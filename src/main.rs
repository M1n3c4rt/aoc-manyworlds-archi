use std::{cmp::{min}, collections::{HashMap, HashSet}, fmt::Debug, io::{Write, stdout}, process::exit};
use rand::{rng, seq::{IndexedRandom, SliceRandom}};
use crossterm::{self, cursor::{Hide, MoveTo, Show}, event::{Event, KeyCode, KeyEvent, read}, execute, queue, style::{Color::Rgb, Print, ResetColor, SetBackgroundColor, SetForegroundColor}, terminal::{Clear, ClearType, EnterAlternateScreen, disable_raw_mode, enable_raw_mode, size}};

fn main() -> std::io::Result<()> {
    let (grid,_logic) = Grid::generate_grid();
    let mut initstate = GridState {grid : grid,player : 0,players : [(39,39),(41,39),(39,41),(41,41)],keys : HashSet::new()};
    
    enable_raw_mode()?;
    execute!(
        stdout(),
        EnterAlternateScreen,
        SetBackgroundColor(Rgb{r:0,b:0,g:0}),
        Clear(ClearType::All),
        Hide,
    )?;
    initstate.draw()?;
    loop {
        match read()? {
            Event::Key(k) => {process_key(&mut initstate, k)?;}
            Event::Resize(_,_) => {initstate.draw()?;}
            _ => continue
        }
    }
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum Cell {
    Wall,
    Empty,
    Player(i16),
    Key(char),
    Door(char),
}

impl Cell {
    fn to_char(self) -> char {
        match self {
            Cell::Wall => '#',
            Cell::Empty => ' ',
            Cell::Player(n) => char::from_digit(n as u32, 10).unwrap_or('?'),
            Cell::Door(c) => c.to_ascii_uppercase(),
            Cell::Key(c) => c
        }
    }
}

struct Grid{ cart : Vec<Vec<Cell>>, tree : HashMap<(i16,i16), Option<(i16,i16)>> }

impl Grid {
    fn generate_grid() -> (Grid,HashMap<char, Vec<char>>) {
        let mut grid: Grid = Grid {cart : vec![vec![Cell::Wall; 81]; 81], tree : HashMap::new()};
        
        for (xs,ys) in [(41,41),(39,41),(41,39),(39,39)] {
            grid.carve(xs,ys,xs==41,ys==41);
            grid.cart[ys as usize][xs as usize] = Cell::Player((xs==41) as i16 + 2 * (ys==41) as i16);
        }
        
        let mut placement: (Option<(HashMap<char, (i16,i16)>,HashMap<(i16,i16), char>,HashMap<char, Vec<char>>)>,usize) = (None,0);
        let mut i = 0;
        while i < 100 || placement.0.is_none() {
            let mut nodes : Vec<&(i16, i16)> = grid.tree.keys().filter(|x| ![(41,41),(39,41),(41,39),(39,39)].contains(x)).collect();
            nodes.shuffle(&mut rng());
            let mut iseq = nodes.into_iter();
            let mut keymap: HashMap<char, (i16,i16)> = HashMap::new();
            let mut doormap: HashMap<(i16,i16), char> = HashMap::new();
            
            for c in 'a'..='z' {
                let n1 = iseq.next().unwrap(); // ⎫
                // ⎬ unwrap justification: the length of iseq is guaranteed to be more than twice the length of 'a'..'z'
                let n2 = iseq.next().unwrap(); // ⎭
                
                let mut current = n1;
                let mut behind = false;
                loop {
                    match grid.tree.get(current) {
                        None => break,
                        Some(n_) => {
                            match n_ {
                                None => break,
                                Some(n) => if n == n2 {behind = true; break} else {current = n}
                            }
                        }
                    }
                }
                
                keymap.insert(c, *if behind {n2} else {n1});
                doormap.insert(*if behind {n1} else {n2}, c);
            }
            
            let mut logic: HashMap<char, Vec<char>> = HashMap::new();
            for (k,v) in &keymap {
                let mut pathback = Vec::new();
                let mut current = v;
                loop {
                    match grid.tree.get(&current) {
                        None => break,
                        Some(new) => {
                            match new {
                                None => break,
                                Some(n) => {
                                    match doormap.get(n) {
                                        None => {}
                                        Some(d) => pathback.push(*d)
                                    }
                                    current = n;
                                },
                            }
                        }
                    }
                }
                
                logic.insert(*k, pathback);
            }
            
            fn consistent(logic : &HashMap<char, Vec<char>>) -> bool {
                let mut finished = HashSet::new();
                loop {
                    let mut possible = false;
                    for (k,v) in logic {
                        if v.iter().all(|x| finished.contains(x)) {
                            if !finished.contains(k) {possible = true};
                            finished.insert(*k);
                        }
                    }
                    if !possible {return false};
                    if finished.len() == 26 {
                        return true
                    }
                }
            }
            
            if consistent(&logic) {
                let sum : usize = logic.iter().map(|(_,v)| v.len()).sum();
                if sum > placement.1 {
                    placement = (Some((keymap,doormap,logic)),sum)
                }
            }
            i += 1;
        }
        
        let (keymap,doormap,logic) = placement.0.unwrap(); // unwrap justification: the above loop will not terminate with placement.0 == None
        for (c,(x,y)) in keymap { grid.cart[y as usize][x as usize] = Cell::Key(c) }
        for ((x,y),c) in doormap { grid.cart[y as usize][x as usize] = Cell::Door(c) }
        (grid,logic)
    }
    
    fn _to_string(self : &Grid) -> String {
        let mut s = String::new();
        for line in &self.cart {
            for c in line {
                s.push(c.to_char());
            }
            s.push('\n')
        }
        s
    }
    
    fn carve(&mut self, x : i16, y : i16, dx : bool, dy : bool) {
        self.cart[y as usize][x as usize] = Cell::Empty;
        let mut stack: Vec<(i16,i16)> = Vec::new();
        stack.push((x,y));
        self.tree.insert((x,y), None);
        loop {
            let cur = stack.pop();
            match cur {
                None => break,
                Some ((xc,yc)) => {
                    let valid_neighbour = |&&(xn,yn) : &&(i16,i16)| {
                        self.cart.get((2*yn-yc) as usize).and_then(|v| v.get((2*xn-xc) as usize)) == Some (&Cell::Wall) &&
                        (xn == xc || ((xn>x) == dx)) &&
                        (yn == yc || ((yn>y) == dy))
                    };
                    let candidates = [(xc+1,yc),(xc,yc+1),(xc-1,yc),(xc,yc-1)];
                    let neighbours : Vec<(i16, i16)> = candidates.iter().filter(valid_neighbour).copied().collect();
                    match neighbours.choose(&mut rng()) {
                        None => continue,
                        Some (&(xn,yn)) => {
                            stack.push((xc,yc));
                            stack.push((2*xn-xc,2*yn-yc));
                            self.tree.insert((xn,yn),Some ((xc,yc)));
                            self.tree.insert((2*xn-xc,2*yn-yc), Some ((xn,yn)));
                            self.cart[yn as usize][xn as usize] = Cell::Empty;
                            self.cart[(2*yn-yc) as usize][(2*xn-xc) as usize] = Cell::Empty;
                        } 
                    }
                }
            }
        }
    }
    
}
struct GridState { grid : Grid, player : usize, players : [(i16,i16); 4], keys : HashSet<char>}

impl GridState {
    fn draw(&self) -> std::io::Result<()> {
        let offset = 27;
        let (xp,yp) = self.players[self.player];
        let (cols,rows) = size()?;
        let width = min(rows as i16,cols as i16 - offset - 1);
        eprintln!("{width} {rows} {cols}");
        let (xs,ys) = (offset + width/2,width/2);

        let mut keystring = String::new();
        for c in 'a'..='z' {
            keystring.push(if self.keys.contains(&c) {c} else {' '})
        }

        execute!(
            stdout(),
            ResetColor,
            //Clear(ClearType::All),
            SetBackgroundColor(Rgb{r:0,g:0,b:0}),
            MoveTo(0,0),
            Print("[wasd]/arrows to move"),
            MoveTo(0,1),
            Print("[c] to change player"),
            MoveTo(0,2),
            Print("[q] to quit"),
            MoveTo(0,3),
            Print("keys collected:"),
            MoveTo(0,4),
            Print(keystring),
        )?;

        for y in 0..width {
            for x in offset..offset+width {

                let distfactor: f64 = (((x-xs) as f64).powf(2.0)+((y-ys) as f64).powf(2.0)).max(1.0).min(255.0);
                let k: f64 = 3.9;
                let col_from = |n:f64| Rgb{r:(k*n/distfactor) as u8, g:(k*n/distfactor) as u8, b:(k*n/distfactor) as u8};
                
                if x-(xs as i16)+xp < 0 || y-(ys as i16)+yp < 0 {continue}
                let cell = self.grid.cart.get((y-(ys as i16)+yp) as usize).and_then(|v| v.get((x-(xs as i16)+xp) as usize)).unwrap_or(&Cell::Wall);
                let col = match cell {
                    Cell::Player(n) => if self.player == *n as usize {col_from(255.0)} else {col_from(128.0)}
                    Cell::Wall | Cell::Empty => col_from(64.0),
                    Cell::Key(_) | Cell::Door(_) => col_from(255.0),
                };
                queue!(
                    stdout(),
                    MoveTo(x as u16,y as u16),
                    SetForegroundColor(col),
                    Print(cell.to_char())
                )?;
            }
        }
        stdout().flush()?;
        Ok(())
    }

    fn move_player(&mut self, dx : i16, dy : i16) -> std::io::Result<()> {
        let (xp,yp) = self.players[self.player];
        if xp+dx < 0 || yp+dy < 0 {return Ok(())}
        let target = self.grid.cart.get((yp+dy) as usize).and_then(|v| v.get((xp+dx) as usize));
        match target {
            None => return Ok(()),
            Some(n) => match n {
                Cell::Wall | Cell::Door(_) => return Ok(()),
                Cell::Empty | Cell::Player(_) => {
                    self.grid.cart[yp as usize][xp as usize] = Cell::Empty;
                    self.grid.cart[(yp+dy) as usize][(xp+dx) as usize] = Cell::Player(self.player as i16);
                    self.players[self.player] = (xp+dx,yp+dy)
                }
                Cell::Key(c) => {
                    self.keys.insert(*c);
                    self.remove_door(*c);
                    self.grid.cart[yp as usize][xp as usize] = Cell::Empty;
                    self.grid.cart[(yp+dy) as usize][(xp+dx) as usize] = Cell::Player(self.player as i16);
                    self.players[self.player] = (xp+dx,yp+dy)
                }
            }
        }
        self.draw()?;
        Ok(())
    }

    fn remove_door(&mut self, c : char) {
        for y in 0..self.grid.cart.len() {
            for x in 0..self.grid.cart[y].len() {
                if self.grid.cart[y][x] == Cell::Door(c) {
                    self.grid.cart[y][x] = Cell::Empty;
                }
            }
        }
    }
}

fn process_key(state : &mut GridState, k : KeyEvent) -> std::io::Result<()> {
    if !k.is_press() {return Ok(())};
    match k.code {
        KeyCode::Char('q') => {
            execute!(
                stdout(),
                Clear(ClearType::All),
                MoveTo(0,0),
                Show
            )?;
            disable_raw_mode()?;
            exit(0);}
        KeyCode::Char('c') => {
            state.player += 1;
            state.player %= 4;
            state.draw()
        }
        KeyCode::Char('w') | KeyCode::Up => state.move_player(0, -1),
        KeyCode::Char('a') | KeyCode::Left => state.move_player(-1, 0),
        KeyCode::Char('s') | KeyCode::Down => state.move_player(0, 1),
        KeyCode::Char('d') | KeyCode::Right => state.move_player(1, 0),
        _ => Ok(())
    }
}
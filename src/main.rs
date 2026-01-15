use archipelago_rs::{
    client::{ArchipelagoClient, ArchipelagoError},
    protocol::{
        ClientMessage, ClientStatus, Connected, DataStorageOperation, Get, ItemsHandlingFlags,
        NetworkItem, NetworkItemFlags, Retrieved, RichMessageColor, RichMessagePart, RichPrint,
        ServerMessage, Set,
    },
};
use crossterm::{
    self,
    cursor::{Hide, MoveTo, Show},
    event::{Event, EventStream, KeyCode, KeyEvent, read},
    execute, queue,
    style::{
        Color::{self, Rgb},
        Print, ResetColor, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{
        Clear, ClearType, DisableLineWrap, EnterAlternateScreen, disable_raw_mode, enable_raw_mode,
        size,
    },
};
use futures_util::{FutureExt, StreamExt};
use rand::{Rng, SeedableRng, rng, seq::IndexedRandom};
use rand_chacha::ChaCha8Rng;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    env::args,
    fmt::Debug,
    io::{Write, stdout},
    iter::repeat_n,
    process::exit,
    sync::Arc,
};
use tokio::{select, time::error::Elapsed};

#[tokio::main]
async fn main() -> Result<(), StrError> {
    let mut rc: std::env::Args = args();
    let mut argmap = HashMap::new();
    let _ = rc.next();
    while let Some(opt) = rc.next() {
        match &opt[..] {
            "--help" | "--singleplayer" => {
                argmap.insert(opt, String::new());
            }
            "--url" | "--password" | "--slot" | "--seed" => match rc.next() {
                Some(s) => {
                    argmap.insert(opt, s);
                }
                None => {
                    return Err(StrError {
                        msg: format!("expected value after {opt}"),
                    });
                }
            },
            _ => {
                return Err(StrError {
                    msg: format!("unrecognized flag {opt}"),
                });
            }
        }
    }
    if argmap.contains_key("--help") {
        println!(concat!(
            "Join a multiworld:\n",
            "aoc-manyworlds --url <url> [--password <password>] --slot <slot>\n\n",
            "Play without joining a multiworld:\n",
            "aoc-manyworlds --singleplayer [--seed <seed>]\n\n",
            "Print help\n",
            "aoc-manyworlds --help"
        ));
    } else if argmap.contains_key("--singleplayer") {
        match argmap.get("--seed") {
            Some(n) => {
                if let Ok(seed) = n.parse() {
                    start_singleplayer(seed)?
                } else {
                    return Err(StrError {
                        msg: "seed must be a number!".to_string(),
                    });
                }
            }
            None => start_singleplayer(rng().random())?,
        };
    } else if let Some(url) = argmap.get("--url")
        && let Some(slot) = argmap.get("--slot")
    {
        let password = argmap.get("--password");
        start_multiplayer(url.clone(), slot.clone(), password.map(|x| x.as_str())).await?;
    } else {
        return Err(StrError {
            msg: "Invalid syntax. Try \"aoc-manyworlds --help\".".to_string(),
        });
    }
    Ok(())
}

struct StrError {
    msg: String,
}
impl Debug for StrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl From<std::io::Error> for StrError {
    fn from(value: std::io::Error) -> Self {
        Self {
            msg: format!("{}", value),
        }
    }
}
impl From<ArchipelagoError> for StrError {
    fn from(value: ArchipelagoError) -> Self {
        Self {
            msg: format!("{}", value),
        }
    }
}
impl From<Elapsed> for StrError {
    fn from(value: Elapsed) -> Self {
        Self {
            msg: format!("{}", value),
        }
    }
}

fn start_singleplayer(seed: u64) -> Result<(), StrError> {
    let (grid, _logic) = Grid::generate_grid(seed);
    let mut initstate = GridState {
        grid,
        player: 0,
        players: [(39, 39), (41, 39), (39, 41), (41, 41)],
        keys: HashSet::new(),
        msgs: repeat_n(RichPrint::message(String::new()), 5).collect(),
        finished: false,
    };

    enable_raw_mode()?;
    execute!(
        stdout(),
        EnterAlternateScreen,
        SetBackgroundColor(Rgb { r: 0, g: 0, b: 0 }),
        Clear(ClearType::All),
        Hide,
    )?;
    loop {
        initstate.draw(&"???".to_string())?;
        match read()? {
            Event::Key(k) => {
                if let Some(k) = initstate.process_key(k)? {
                    initstate.use_key(&k);
                }
            }
            _ => continue,
        }

        if initstate.keys.len() == 26 {
            initstate.finished = true;
        }
    }
}

async fn start_multiplayer(
    url: String,
    slot: String,
    password: Option<&str>,
) -> Result<(), StrError> {
    let mut con: ArchipelagoClient<Value> = match ArchipelagoClient::new(&url).await {
        Ok(c) => c,
        Err(ArchipelagoError::NetworkError(e)) => {
            descriptive_network_error(e);
        }
        Err(e) => {
            return Err(StrError::from(e));
        }
    };
    con = ArchipelagoClient::with_data_package(&url, Some(con.room_info().games.clone())).await?;

    let con_package: Connected<Value> = con
        .connect(
            "Advent of Code 2019 Day 18 Part 2",
            &slot,
            password,
            ItemsHandlingFlags::all(),
            vec!["AP".to_string()],
        )
        .await?;
    let data_package = con
        .data_package()
        .ok_or(StrError {
            msg: "could not retrieve data package".to_string(),
        })?
        .clone();
    let _logic: HashMap<char, Vec<char>> = logic_from_connected(&con_package)?; //store here for sanity
    let grid = grid_from_connected(&con_package)?;

    con.send(ClientMessage::Set(Set {
        key: slot.clone() + "_players",
        default: seriaize_players([(39, 39), (41, 39), (39, 41), (41, 41)]),
        want_reply: false,
        operations: vec![DataStorageOperation::Remove(json!([-1, -1]))],
    }))
    .await?;
    con.send(ClientMessage::Set(Set {
        key: slot.clone() + "_keystring",
        default: Value::Array(Vec::new()),
        want_reply: false,
        operations: vec![DataStorageOperation::Remove(json!('?'))],
    }))
    .await?;
    con.send(ClientMessage::Get(Get {
        keys: vec![slot.clone() + "_keystring"],
    }))
    .await?;
    con.send(ClientMessage::Get(Get {
        keys: vec![slot.clone() + "_players"],
    }))
    .await?;

    let mut initstate = GridState {
        grid,
        player: 0,
        players: [(39, 39), (41, 39), (39, 41), (41, 41)],
        keys: HashSet::new(),
        msgs: repeat_n(RichPrint::message(String::new()), 5).collect(),
        finished: false,
    };

    enable_raw_mode()?;
    execute!(
        stdout(),
        EnterAlternateScreen,
        SetBackgroundColor(Rgb { r: 0, g: 0, b: 0 }),
        Clear(ClearType::All),
        DisableLineWrap,
        Hide,
    )?;

    let mut reader = EventStream::new();
    let mut keysretrieved = false;
    let mut playersretrieved = false;

    loop {
        if keysretrieved && playersretrieved {
            initstate.draw(&slot)?;
        }

        let event = reader.next().fuse();

        select! {
            maybe_event = event => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    if let Some(k) = initstate.process_key(key)? {
                        con.location_checks(vec![k as i64]).await?;
                        con.send(ClientMessage::Set(Set {
                            key: slot.clone()+"_keystring",
                            default: Value::Array(vec![Value::String(k.to_string())]),
                            want_reply: false,
                            operations: vec![DataStorageOperation::Update(json!([k.to_string()]))]
                        })).await?;
                    }
                    con.send(ClientMessage::Set(Set {
                        key: slot.clone()+"_players",
                        default: seriaize_players(initstate.players),
                        want_reply: false,
                        operations: vec![DataStorageOperation::Replace(seriaize_players(initstate.players))]
                    })).await?;
                }
            },
            server_msg = con.recv() => {
                match server_msg {
                    Ok(Some(ServerMessage::ReceivedItems(items))) => {
                        let is = items.items;
                        initstate.add_items(is)?;
                    },
                    Ok(Some(ServerMessage::Print(msg))) => {
                        initstate.msgs.pop_front();
                        initstate.msgs.push_back(RichPrint::message(msg.text));
                    },
                    Ok(Some(ServerMessage::RichPrint(mut msg))) => {
                        initstate.msgs.pop_front();
                        msg.add_names(&con_package, &data_package);
                        initstate.msgs.push_back(msg);
                    },
                    Ok(Some(ServerMessage::Retrieved(items))) => {
                        if let Ok(keys) = keystring_from_storage(&value_from_singleton(&items)?) {
                            for key in keys {initstate.remove_cell(Cell::Key(key))};
                            keysretrieved = true;
                        }
                        if let Ok(players) = players_from_storage(&value_from_singleton(&items)?) {
                            for i in 0..4 {
                                initstate.remove_cell(Cell::Player(i));
                            }
                            initstate.players = players;
                            for ((x,y),n) in initstate.players.iter().zip(0..4) {
                                initstate.grid.cart[*y as usize][*x as usize] = Cell::Player(n);
                            }
                            playersretrieved = true;
                        }
                    }
                    _ => {}
                }
            }
        };
        if initstate.keys.len() == 26 {
            con.status_update(ClientStatus::ClientGoal).await?;
            initstate.finished = true;
        }
    }
}

fn logic_from_connected(
    package: &Connected<Value>,
) -> Result<HashMap<char, Vec<char>>, ArchipelagoError> {
    let data = if let Value::Object(data) = &package.slot_data {
        data
    } else {
        return Err(ArchipelagoError::IllegalResponse {
            expected: "object",
            received: "non-object JSON type",
        });
    };

    let m = if let Some(Value::Object(m)) = data.get("logic") {
        m
    } else {
        return Err(ArchipelagoError::IllegalResponse {
            expected: "logic",
            received: "nothing",
        });
    };

    let mut logic = HashMap::new();
    for (k, v) in m {
        let mut ls = Vec::new();
        let ks = if let Value::Array(ks) = v {
            ks
        } else {
            return Err(ArchipelagoError::IllegalResponse {
                expected: "array",
                received: "non-array JSON type",
            });
        };

        for k in ks {
            if let Value::String(k) = k {
                ls.push(k.chars().next().ok_or(ArchipelagoError::IllegalResponse {
                    expected: "nonempty string",
                    received: "empty string",
                })?);
            } else {
                return Err(ArchipelagoError::IllegalResponse {
                    expected: "string",
                    received: "non-string JSON type",
                });
            }
        }
        logic.insert(
            k.chars().next().ok_or(ArchipelagoError::IllegalResponse {
                expected: "nonempty string",
                received: "empty string",
            })?,
            ls,
        );
    }
    Ok(logic)
}

fn grid_from_connected(package: &Connected<Value>) -> Result<Grid, ArchipelagoError> {
    let data = if let Value::Object(data) = &package.slot_data {
        data
    } else {
        return Err(ArchipelagoError::IllegalResponse {
            expected: "object",
            received: "non-object JSON type",
        });
    };

    let rows = if let Some(Value::Array(rows)) = data.get("grid") {
        rows
    } else {
        return Err(ArchipelagoError::IllegalResponse {
            expected: "grid",
            received: "nothing",
        });
    };

    let mut cart = Vec::new();
    for row in rows {
        let mut cellrow = Vec::new();
        let r = if let Value::Array(r) = row {
            r
        } else {
            return Err(ArchipelagoError::IllegalResponse {
                expected: "array",
                received: "non-array JSON type",
            });
        };

        for cell in r {
            if let Value::String(c) = cell {
                cellrow.push(Cell::to_cell(c.chars().next().ok_or(
                    ArchipelagoError::IllegalResponse {
                        expected: "nonempty string",
                        received: "empty string",
                    },
                )?)?)
            }
        }
        cart.push(cellrow);
    }
    Ok(Grid {
        cart,
        tree: HashMap::new(),
    }) // no need to populate tree because we don't ever use it outside of singleplayer generation
}

fn keystring_from_storage(storage: &Value) -> Result<HashSet<char>, ArchipelagoError> {
    let keystring = if let Value::Array(s) = &storage {
        s
    } else {
        return Err(ArchipelagoError::IllegalResponse {
            expected: "array",
            received: "non-array JSON type",
        });
    };
    let mut keyset = HashSet::new();
    for elem in keystring {
        if let Value::String(s) = elem {
            keyset.insert(s.chars().next().ok_or(ArchipelagoError::IllegalResponse {
                expected: "single character string",
                received: "empty string",
            })?);
        } else {
            return Err(ArchipelagoError::IllegalResponse {
                expected: "string",
                received: "non-string JSON type",
            });
        }
    }

    Ok(keyset)
}

fn players_from_storage(storage: &Value) -> Result<[(i16, i16); 4], ArchipelagoError> {
    let players = if let Value::Array(v) = &storage {
        v
    } else {
        return Err(ArchipelagoError::IllegalResponse {
            expected: "array",
            received: "non-array JSON type",
        });
    };

    let mut player_array = [(0, 0), (0, 0), (0, 0), (0, 0)];
    for (player, i) in players.iter().take(4).zip(0..4) {
        let pair = if let Value::Array(p) = player {
            p
        } else {
            return Err(ArchipelagoError::IllegalResponse {
                expected: "array",
                received: "non-array JSON type",
            });
        };
        let coords = match &pair[..] {
            [Value::Number(x), Value::Number(y)] => match (x.as_i64(), y.as_i64()) {
                (Some(x), Some(y)) => (x as i16, y as i16),
                _ => {
                    return Err(ArchipelagoError::IllegalResponse {
                        expected: "integer",
                        received: "non-integer JSON type",
                    });
                }
            },
            _ => {
                return Err(ArchipelagoError::IllegalResponse {
                    expected: "integer",
                    received: "non-integer JSON type",
                });
            }
        };
        player_array[i] = coords;
    }
    if player_array != [(0, 0), (0, 0), (0, 0), (0, 0)] {
        Ok(player_array)
    } else {
        Err(ArchipelagoError::IllegalResponse {
            expected: "integer",
            received: "non-integer JSON type",
        })
    }
}

fn seriaize_players([(x1, y1), (x2, y2), (x3, y3), (x4, y4)]: [(i16, i16); 4]) -> Value {
    json!([[x1, y1], [x2, y2], [x3, y3], [x4, y4]])
}

fn value_from_singleton(storage: &Retrieved) -> Result<Value, ArchipelagoError> {
    let obj = if let Value::Object(o) = &storage.keys {
        o
    } else {
        return Err(ArchipelagoError::IllegalResponse {
            expected: "object",
            received: "non-object JSON type",
        });
    };
    obj.values()
        .next()
        .ok_or(ArchipelagoError::IllegalResponse {
            expected: "nonempty object",
            received: "empty object",
        })
        .cloned()
}

fn descriptive_network_error(err: tungstenite::Error) -> ! {
    match err {
        tungstenite::Error::ConnectionClosed => {
            eprintln!("connection closed");
            exit(1)
        }
        tungstenite::Error::AlreadyClosed => {
            eprintln!("already closed");
            exit(1)
        }
        tungstenite::Error::Io(e) => {
            eprintln!("io: {e}");
            exit(1)
        }
        tungstenite::Error::Tls(e) => {
            eprintln!("tls: {e}");
            exit(1)
        }
        tungstenite::Error::Capacity(e) => {
            eprintln!("capacity: {e}");
            exit(1)
        }
        tungstenite::Error::Protocol(e) => {
            eprintln!("protocol: {e}");
            exit(1)
        }
        tungstenite::Error::WriteBufferFull(e) => {
            eprintln!("write buffer full: {e}");
            exit(1)
        }
        tungstenite::Error::Utf8(e) => {
            eprintln!("utf-8: {e}");
            exit(1)
        }
        tungstenite::Error::AttackAttempt => {
            eprintln!("attack attempt");
            exit(1)
        }
        tungstenite::Error::Url(e) => {
            eprintln!("url error: {e}");
            exit(1)
        }
        tungstenite::Error::Http(e) => {
            eprintln!("http error: {e:?}");
            exit(1)
        }
        tungstenite::Error::HttpFormat(e) => {
            eprintln!("http format error: {e}");
            exit(1)
        }
    };
}

fn rich_print(slot: &String, msg: RichPrint) -> Result<(), StrError> {
    fn col_from_flags(flags: &NetworkItemFlags) -> Color {
        if flags.contains(NetworkItemFlags::PROGRESSION) {
            Rgb {
                r: 175,
                g: 153,
                b: 239,
            }
        } else if flags.contains(NetworkItemFlags::USEFUL) {
            Rgb {
                r: 109,
                g: 139,
                b: 232,
            }
        } else if flags.contains(NetworkItemFlags::TRAP) {
            Rgb {
                r: 250,
                g: 128,
                b: 114,
            }
        } else {
            Color::Cyan
        }
    }

    for part in msg.data() {
        match part {
            RichMessagePart::PlayerId { id: _, name } => {
                let name = name.clone().unwrap_or(Arc::new("???".to_string()));
                execute!(
                    stdout(),
                    SetForegroundColor(if name == Arc::new(slot.clone()) {
                        Color::Magenta
                    } else {
                        Color::Yellow
                    }),
                    Print(name),
                )?;
            }
            RichMessagePart::PlayerName { text } => {
                let name = text;
                execute!(
                    stdout(),
                    SetForegroundColor(if name == slot {
                        Color::Magenta
                    } else {
                        Color::Yellow
                    }),
                    Print(name),
                )?;
            }
            RichMessagePart::ItemId {
                id: _,
                flags,
                player: _,
                name,
            } => {
                let name = name.clone().unwrap_or(Arc::new("???".to_string()));
                execute!(
                    stdout(),
                    SetForegroundColor(col_from_flags(flags)),
                    Print(name),
                )?;
            }
            RichMessagePart::ItemName {
                text,
                flags,
                player: _,
            } => {
                let name = text;
                execute!(
                    stdout(),
                    SetForegroundColor(col_from_flags(flags)),
                    Print(name),
                )?;
            }
            RichMessagePart::LocationId {
                id: _,
                player: _,
                name,
            } => {
                let name = name.clone().unwrap_or(Arc::new("???".to_string()));
                execute!(stdout(), SetForegroundColor(Color::Green), Print(name),)?;
            }
            RichMessagePart::LocationName { text, player: _ } => {
                let name = text;
                execute!(stdout(), SetForegroundColor(Color::Green), Print(name),)?;
            }
            RichMessagePart::EntranceName { text } => {
                let name = text;
                execute!(stdout(), SetForegroundColor(Color::Blue), Print(name),)?;
            }
            RichMessagePart::Color { text, color } => {
                execute!(
                    stdout(),
                    SetForegroundColor(match color {
                        RichMessageColor::Red => Color::Red,
                        RichMessageColor::Green => Color::Green,
                        RichMessageColor::Yellow => Color::Yellow,
                        RichMessageColor::Blue => Color::Blue,
                        RichMessageColor::Magenta => Color::Magenta,
                        RichMessageColor::Cyan => Color::Cyan,
                        _ => Color::White,
                    }),
                    Print(text),
                )?;
            }
            RichMessagePart::Text { text } => {
                execute!(
                    stdout(),
                    ResetColor,
                    SetBackgroundColor(Rgb { r: 0, g: 0, b: 0 }),
                    Print(text)
                )?;
            }
        }
    }
    execute!(stdout(), Print(" ".repeat(2000)),)?;
    Ok(())
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
            Cell::Key(c) => c,
        }
    }

    fn to_cell(c: char) -> Result<Self, ArchipelagoError> {
        match c {
            '#' => Ok(Cell::Wall),
            ' ' | '.' => Ok(Cell::Empty),
            k => {
                if k.is_numeric() {
                    Ok(Cell::Player(
                        k.to_digit(10).ok_or(ArchipelagoError::IllegalResponse {
                            expected: "digit",
                            received: "non-digit",
                        })? as i16,
                    ))
                } else if k.is_ascii_uppercase() {
                    Ok(Cell::Door(k.to_ascii_lowercase()))
                } else if k.is_ascii_lowercase() {
                    Ok(Cell::Key(k))
                } else {
                    Err(ArchipelagoError::IllegalResponse {
                        expected: "input character like #, a, 2, etc",
                        received: "unknown character",
                    })
                }
            }
        }
    }
}

struct Grid {
    cart: Vec<Vec<Cell>>,
    tree: HashMap<(i16, i16), Option<(i16, i16)>>,
}
type KeyMap = HashMap<char, (i16, i16)>;
type DoorMap = HashMap<(i16, i16), char>;
type Logic = HashMap<char, Vec<char>>;

impl Grid {
    fn generate_grid(seed: u64) -> (Grid, Logic) {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut grid: Grid = Grid {
            cart: vec![vec![Cell::Wall; 81]; 81],
            tree: HashMap::new(),
        };

        for (xs, ys) in [(41, 41), (39, 41), (41, 39), (39, 39)] {
            grid.carve(xs, ys, xs == 41, ys == 41, &mut rng);
            grid.cart[ys as usize][xs as usize] =
                Cell::Player((xs == 41) as i16 + 2 * (ys == 41) as i16);
        }

        let mut placement: (Option<(KeyMap, DoorMap, Logic)>, usize) = (None, 0);
        let mut i = 0;

        let mut nodes: Vec<&(i16, i16)> = grid
            .tree
            .keys()
            .filter(|x| ![(41, 41), (39, 41), (41, 39), (39, 39)].contains(x))
            .collect();
        nodes.sort();
        while i < 100 || placement.0.is_none() {
            let mut iseq = nodes.choose_multiple(&mut rng, 52);
            let mut keymap: HashMap<char, (i16, i16)> = HashMap::new();
            let mut doormap: HashMap<(i16, i16), char> = HashMap::new();

            for c in 'a'..='z' {
                let n1 = *iseq.next().unwrap(); // ⎫
                // ⎬ unwrap justification: iseq is guaranteed to be 52 elements long, same as 2 times 'a'..='z'
                let n2 = *iseq.next().unwrap(); // ⎭

                let mut current = n1;
                let mut behind = false;
                while let Some(Some(n)) = grid.tree.get(current) {
                    if n == n2 {
                        behind = true;
                        break;
                    } else {
                        current = n
                    }
                }

                keymap.insert(c, *if behind { n2 } else { n1 });
                doormap.insert(*if behind { n1 } else { n2 }, c);
            }

            let mut logic: HashMap<char, Vec<char>> = HashMap::new();
            for (k, v) in &keymap {
                let mut pathback = Vec::new();
                let mut current = v;
                while let Some(Some(n)) = grid.tree.get(current) {
                    match doormap.get(n) {
                        None => {}
                        Some(d) => pathback.push(*d),
                    }
                    current = n;
                }
                logic.insert(*k, pathback);
            }

            fn consistent(logic: &Logic) -> bool {
                let mut finished = HashSet::new();
                loop {
                    let mut possible = false;
                    for (k, v) in logic {
                        if v.iter().all(|x| finished.contains(x)) {
                            if !finished.contains(k) {
                                possible = true
                            };
                            finished.insert(*k);
                        }
                    }
                    if !possible {
                        return false;
                    };
                    if finished.len() == 26 {
                        return true;
                    }
                }
            }

            if consistent(&logic) {
                let sum: usize = logic.values().map(|v| v.len()).sum();
                if sum > placement.1 {
                    placement = (Some((keymap, doormap, logic)), sum);
                }
            }
            i += 1;
        }

        let (keymap, doormap, logic) = placement.0.unwrap(); // unwrap justification: the above loop will not terminate with placement.0 == None
        for (c, (x, y)) in keymap {
            grid.cart[y as usize][x as usize] = Cell::Key(c)
        }
        for ((x, y), c) in doormap {
            grid.cart[y as usize][x as usize] = Cell::Door(c)
        }
        (grid, logic)
    }

    fn carve(&mut self, x: i16, y: i16, dx: bool, dy: bool, rng: &mut ChaCha8Rng) {
        self.cart[y as usize][x as usize] = Cell::Empty;
        let mut stack: Vec<(i16, i16)> = Vec::new();
        stack.push((x, y));
        self.tree.insert((x, y), None);
        loop {
            let cur = stack.pop();
            match cur {
                None => break,
                Some((xc, yc)) => {
                    let valid_neighbour = |&&(xn, yn): &&(i16, i16)| {
                        self.cart
                            .get((2 * yn - yc) as usize)
                            .and_then(|v| v.get((2 * xn - xc) as usize))
                            == Some(&Cell::Wall)
                            && (xn == xc || ((xn > x) == dx))
                            && (yn == yc || ((yn > y) == dy))
                    };
                    let candidates = [(xc + 1, yc), (xc, yc + 1), (xc - 1, yc), (xc, yc - 1)];
                    let neighbours: Vec<(i16, i16)> =
                        candidates.iter().filter(valid_neighbour).copied().collect();
                    match neighbours.choose(rng) {
                        None => continue,
                        Some(&(xn, yn)) => {
                            stack.push((xc, yc));
                            stack.push((2 * xn - xc, 2 * yn - yc));
                            self.tree.insert((xn, yn), Some((xc, yc)));
                            self.tree.insert((2 * xn - xc, 2 * yn - yc), Some((xn, yn)));
                            self.cart[yn as usize][xn as usize] = Cell::Empty;
                            self.cart[(2 * yn - yc) as usize][(2 * xn - xc) as usize] = Cell::Empty;
                        }
                    }
                }
            }
        }
    }
}
struct GridState {
    grid: Grid,
    player: usize,
    players: [(i16, i16); 4],
    keys: HashSet<char>,
    msgs: VecDeque<RichPrint>,
    finished: bool,
}

impl GridState {
    fn draw(&self, slot: &String) -> Result<(), StrError> {
        let offset = 27;
        let (xp, yp) = self.players[self.player];
        let (cols, rows) = size()?;
        let width = cols as i16 - offset - 1;
        let height = rows as i16 - 6;
        let (xs, ys) = (offset + width / 2, height / 2);

        let mut keystring = String::new();
        for c in 'a'..='z' {
            keystring.push(if self.keys.contains(&c) { c } else { ' ' })
        }

        execute!(
            stdout(),
            ResetColor,
            SetBackgroundColor(Rgb { r: 0, g: 0, b: 0 }),
            MoveTo(0, 0),
            Print("[wasd]/arrows to move"),
            MoveTo(0, 1),
            Print("[c] to change player"),
            MoveTo(0, 2),
            Print("[q] to quit"),
            MoveTo(0, 3),
            Print("keys collected:"),
            MoveTo(0, 4),
            SetForegroundColor(if self.finished {
                Rgb {
                    r: 235,
                    g: 255,
                    b: 108,
                }
            } else {
                Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                }
            }),
            Print(keystring),
            ResetColor,
            SetBackgroundColor(Rgb { r: 0, g: 0, b: 0 }),
            MoveTo(0, height as u16),
        )?;
        rich_print(slot, self.msgs[0].clone())?;
        execute!(stdout(), MoveTo(0, height as u16 + 1))?;
        rich_print(slot, self.msgs[1].clone())?;
        execute!(stdout(), MoveTo(0, height as u16 + 2))?;
        rich_print(slot, self.msgs[2].clone())?;
        execute!(stdout(), MoveTo(0, height as u16 + 3))?;
        rich_print(slot, self.msgs[3].clone())?;
        execute!(stdout(), MoveTo(0, height as u16 + 4))?;
        rich_print(slot, self.msgs[4].clone())?;
        execute!(
            stdout(),
            MoveTo(0, height as u16 + 5),
            SetForegroundColor(if self.finished {
                Rgb {
                    r: 235,
                    g: 255,
                    b: 108,
                }
            } else {
                Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                }
            }),
            Print(if self.finished {
                "You have found all of your keys!"
            } else {
                ""
            })
        )?;

        for y in 0..height {
            for x in offset..offset + width {
                let distfactor: f64 = (((x - xs) as f64).powf(2.0)
                    + ((2 * y - 2 * ys) as f64).powf(2.0))
                .clamp(1.0, 255.0);
                let k: f64 = 3.9;
                let col_from = |n: f64| Rgb {
                    r: (k * n / distfactor) as u8,
                    g: (k * n / distfactor) as u8,
                    b: (k * n / distfactor) as u8,
                };

                let cell = if (y - ys + yp) < 0 || (x - xs + xp) < 0 {
                    &Cell::Wall
                } else {
                    self.grid
                        .cart
                        .get((y - ys + yp) as usize)
                        .and_then(|v| v.get((x - xs + xp) as usize))
                        .unwrap_or(&Cell::Wall)
                };
                let col = match cell {
                    Cell::Player(n) => {
                        if self.player == *n as usize {
                            col_from(255.0)
                        } else {
                            col_from(128.0)
                        }
                    }
                    Cell::Wall | Cell::Empty => col_from(64.0),
                    Cell::Key(_) | Cell::Door(_) => col_from(255.0),
                };
                queue!(
                    stdout(),
                    MoveTo(x as u16, y as u16),
                    SetForegroundColor(col),
                    Print(cell.to_char())
                )?;
            }
        }
        stdout().flush()?;
        Ok(())
    }

    fn move_player(&mut self, dx: i16, dy: i16) -> std::io::Result<Option<char>> {
        let (xp, yp) = self.players[self.player];
        if xp + dx < 0 || yp + dy < 0 {
            return Ok(None);
        }
        let target = self
            .grid
            .cart
            .get((yp + dy) as usize)
            .and_then(|v| v.get((xp + dx) as usize));
        match target {
            None => return Ok(None),
            Some(n) => match n {
                Cell::Wall | Cell::Door(_) => return Ok(None),
                Cell::Empty | Cell::Player(_) => {
                    self.grid.cart[yp as usize][xp as usize] = Cell::Empty;
                    self.grid.cart[(yp + dy) as usize][(xp + dx) as usize] =
                        Cell::Player(self.player as i16);
                    self.players[self.player] = (xp + dx, yp + dy)
                }
                Cell::Key(c) => {
                    let k = *c;
                    self.grid.cart[yp as usize][xp as usize] = Cell::Empty;
                    self.grid.cart[(yp + dy) as usize][(xp + dx) as usize] =
                        Cell::Player(self.player as i16);
                    self.players[self.player] = (xp + dx, yp + dy);
                    return Ok(Some(k));
                }
            },
        }
        Ok(None)
    }

    fn use_key(&mut self, c: &char) {
        let k = *c;
        self.keys.insert(k);
        self.remove_cell(Cell::Door(k));
    }

    fn remove_cell(&mut self, c: Cell) {
        for y in 0..self.grid.cart.len() {
            for x in 0..self.grid.cart[y].len() {
                if self.grid.cart[y][x] == c {
                    self.grid.cart[y][x] = Cell::Empty;
                }
            }
        }
    }

    fn process_key(&mut self, k: KeyEvent) -> std::io::Result<Option<char>> {
        if !k.is_press() {
            return Ok(None);
        };
        match k.code {
            KeyCode::Char('q') => {
                execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0), Show)?;
                disable_raw_mode()?;
                exit(0);
            }
            KeyCode::Char('c') => {
                self.player += 1;
                self.player %= 4;
                Ok(None)
            }
            KeyCode::Char('w') | KeyCode::Up => self.move_player(0, -1),
            KeyCode::Char('a') | KeyCode::Left => self.move_player(-1, 0),
            KeyCode::Char('s') | KeyCode::Down => self.move_player(0, 1),
            KeyCode::Char('d') | KeyCode::Right => self.move_player(1, 0),
            _ => Ok(None),
        }
    }

    fn add_items(&mut self, received: Vec<NetworkItem>) -> Result<(), StrError> {
        for item in received {
            if let Some(x) = char::from_u32(item.item as u32) {
                self.use_key(&x);
            }
        }
        Ok(())
    }
}

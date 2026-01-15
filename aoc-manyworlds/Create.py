class Cell:
    cellType = "Wall"
    player = 0
    keyid = 'a'

    def __init__(self,ctype,pl=0,id='a'):
        self.cellType = ctype
        self.player = pl
        self.keyid = id

    def to_char(self):
        if self.cellType == "Wall":
            return '#'
        elif self.cellType == "Empty":
            return ' '
        elif self.cellType == "Player":
            return str(self.player)
        elif self.cellType == "Door":
            return self.keyid.upper()
        elif self.cellType == "Key":
            return self.keyid
        else:
            return '?'

class Grid:
    cart = []
    tree: dict()#HashMap<(i16, i16), Option<(i16, i16)>>,

    def __init__(self):
        self.cart = []
        for i in range(81):
            row = []
            for j in range(81):
                row.append(Cell("Wall"))
            self.cart.append(row)
        self.tree = dict()

    def carve(self, world, x, y, dx, dy):
        self.cart[y][x] = Cell("Empty")
        stack = []
        stack.append((x, y))
        self.tree[(x, y)] = None
        while True:
            if len(stack) == 0:
                break
            else:
                (xc, yc) = stack.pop()
                def valid_neighbour(xn,yn):
                    inBoundsX = 0 <= 2 * xn - xc < 81
                    inBoundsY = 0 <= 2 * yn - yc < 81
                    dirX = xn == xc or ((xn > x) == dx)
                    dirY = yn == yc or ((yn > y) == dy)
                    if not (inBoundsX and inBoundsY and dirX and dirY): return False
                    return self.cart[2*yn-yc][2*xn-xc].cellType == "Wall"

                candidates = [(xc + 1, yc), (xc, yc + 1), (xc - 1, yc), (xc, yc - 1)]
                neighbours = list(filter(lambda n: valid_neighbour(n[0],n[1]), candidates))
                if len(neighbours) == 0:
                    continue
                else:
                    (xn,yn) = world.random.choice(neighbours)
                    stack.append((xc, yc))
                    stack.append((2 * xn - xc, 2 * yn - yc))
                    self.tree[(xn, yn)] = (xc, yc)
                    self.tree[(2 * xn - xc, 2 * yn - yc)] = (xn, yn)
                    self.cart[yn][xn] = Cell("Empty")
                    self.cart[2 * yn - yc][2 * xn - xc] = Cell("Empty")

def generate_grid(world):
    grid = Grid()

    for (xs, ys) in [(41, 41), (39, 41), (41, 39), (39, 39)]:
        grid.carve(world, xs, ys, xs == 41, ys == 41)
        grid.cart[ys][xs] = Cell("Player",pl=int(xs == 41) + 2 * int(ys == 41))

    placement = (None, 0)
    i = 0

    nodes = sorted(filter(lambda x: x not in [(41, 41), (39, 41), (41, 39), (39, 39)],grid.tree.keys()))
    while i < 100 or placement[0] is None:
        iseq = iter(world.random.sample(nodes,52))
        keymap = dict()
        doormap = dict()

        for c in 'abcdefghijklmnopqrstuvwxyz':
            n1 = next(iseq)
            n2 = next(iseq)

            current = n1
            behind = False
            while current in grid.tree and grid.tree[current] is not None:
                if grid.tree[current] == n2:
                    behind = True
                    break
                else:
                    current = grid.tree[current]

            keymap[c] = n2 if behind else n1
            doormap[n1 if behind else n2] = c

        logic = dict()
        for k, v in keymap.items():
            pathback = []
            current = v
            while current in grid.tree and grid.tree[current] is not None:
                if grid.tree[current] in doormap:
                    pathback.append(doormap[grid.tree[current]])
                current = grid.tree[current]
            
            logic[k] = pathback

        def consistent(logic):
            finished = set()
            while True:
                possible = False
                for k, v in logic.items():
                    if all(x in finished for x in v):
                        if k not in finished:
                            possible = True
                        finished.add(k)
                
                if not possible:
                    return False

                if len(finished) == 26:
                    return True

        if consistent(logic):
            s = sum(map(len,logic.values()))
            if s > placement[1]:
                placement = ((keymap, doormap, logic), s)

        i += 1

    (keymap, doormap, logic) = placement[0]
    for c, (x, y) in keymap.items():
        grid.cart[y][x] = Cell("Key",id=c)
    
    for (x, y), c in doormap.items():
        grid.cart[y][x] = Cell("Door",id=c)
    
    return (grid, logic)

def prettyprint(grid):
    for row in grid.cart:
        for c in row:
            print(c.to_char(),end="")
        print("")

def get_slot_data(world):
    grid, logic = generate_grid(world)
    print(logic)
    prettyprint(grid)
    input("")
    return {"logic":logic,"grid":list(map(lambda r: list(map(lambda c: c.to_char(),r)),grid.cart))}
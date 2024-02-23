use solver::Array2D;

#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Instruction {
    Up,
    Down,
    Left,
    Right,
    PenUp,
    PenDown,
    Goto(u8, u8),
    ToViewArea,
}

fn find_start(unsolvied_grid: &Array2D) -> Option<(u8, isize, isize)> {
    for i in 0..5 {
        for j in 0..5 {
            if unsolvied_grid[(i, j)] != 0 {
                return Some((unsolvied_grid[(i, j)], i, j));
            }
        }
    }
    None
}

fn iterate_around(x: isize, y: isize) -> impl Iterator<Item = ((isize, isize), Instruction)> {
    use Instruction::*;
    [
        ((x + 1, y), Right),
        ((x - 1, y), Left),
        ((x, y + 1), Down),
        ((x, y - 1), Up),
    ]
    .into_iter()
    .filter(|((x, y), _)| *x >= 0 && *y >= 0 && *x < 5 && *y < 5)
}

pub fn pathfind(mut unsolvied_grid: Array2D, solved_grid: &Array2D) -> Vec<Instruction> {
    let mut instructions = Vec::new();

    loop {
        let (col, mut x, mut y) = match find_start(&unsolvied_grid) {
            Some(a) => a,
            None => break,
        };
        unsolvied_grid[(x, y)] = 0;
        let (mut old_x, mut old_y) = (-1, -1);

        instructions.push(Instruction::Goto(x as u8, y as u8));
        loop {
            if let Some(((newx, newy), dir)) = iterate_around(x, y)
                .filter(|(x, _)| solved_grid[*x] == col)
                .filter(|(point, _)| *point != (old_x, old_y))
                .next()
            {
                old_x = x;
                old_y = y;
                x = newx;
                y = newy;

                instructions.push(dir);
            } else {
                unsolvied_grid[(x, y)] = 0;
                break;
            }
        }
    }

    instructions
}

pub use log::trace;
pub use utils::{get_around, Array2D, IndexTy};

mod utils;
pub type Heads = Vec<((IndexTy, IndexTy), (IndexTy, IndexTy))>;

const MAX_SOLVE_ITER: usize = 100;

#[derive(Debug)]
enum StepResult {
    ProgressMade,
    Complete,
    Impossible,

    NoProgress(Vec<(IndexTy, IndexTy)>),
}

pub fn make_grid_and_pairs(
    size: (usize, usize),
    points: &mut impl Iterator<Item = u8>,
) -> (Array2D, Heads) {
    let mut grid = Array2D::new(size.0, size.1);
    let mut heads: Heads = Vec::with_capacity(256);

    let mut x = 0;
    let mut y = 0;
    for (i, point) in points.enumerate() {
        if point > 0 {
            grid.set_abs(i, point);
            match heads.get_mut(point as usize - 1) {
                Some(pair) => {
                    pair.1 = (x, y);
                }
                None => {
                    heads.push(((x, y), (-1, -1)));
                }
            }
        }

        x += 1;
        if x >= size.0 as isize {
            x = 0;
            y += 1;
        }
    }

    (grid, heads)
}

fn next_step(
    current_head: &mut (IndexTy, IndexTy),
    paired_head: &mut (IndexTy, IndexTy),
    grid: &mut Array2D,
) -> StepResult {
    let mut all_possibles_list = Vec::with_capacity(4);

    for pos in get_around(*current_head) {
        if pos == *paired_head {
            return StepResult::Complete;
        }
        if grid[pos] == 0 {
            all_possibles_list.push(pos);
        }
    }

    match all_possibles_list.len() {
        0 => StepResult::Impossible,
        1 => {
            let only_option = all_possibles_list[0];
            grid[only_option] = grid[*current_head];
            *current_head = only_option;
            return StepResult::ProgressMade;
        }
        _ => StepResult::NoProgress(all_possibles_list),
    }
}

pub fn solve(mut grid: Array2D, mut heads: Heads) -> (Array2D, bool) {
    let mut max_iter_count = 0;
    let mut undo_stack: Vec<(Array2D, Heads, Vec<(IndexTy, IndexTy)>, u8, usize, usize)> =
        Vec::new();
    loop {
        max_iter_count += 1;
        if max_iter_count >= MAX_SOLVE_ITER {
            return (grid, false);
        }
        
        let mut progress_made = false;
        let mut complete_count = 0;
        let mut impossible = false;
        let mut guessing_target = None;

        for (i, (h1, h2)) in heads.iter_mut().enumerate() {
            for (j, result) in [next_step(h1, h2, &mut grid), next_step(h2, h1, &mut grid)]
                .into_iter()
                .enumerate()
            {
                match result {
                    StepResult::ProgressMade => progress_made = true,
                    StepResult::Complete => complete_count += 1,
                    StepResult::Impossible => {
                        impossible = true;
                        break;
                    }
                    StepResult::NoProgress(data) => guessing_target = Some((data, grid[*h1], i, j)),
                }
            }
        }

        trace!("Complete head count: {complete_count}");
        if complete_count >= heads.len() * 2 {
            trace!("Solving iter - complete!");
            // grid is complete!
            return (grid, true);
        }

        if impossible {
            trace!("Impossible grid found");
            if let Some((base_grid, base_heads, split, col, i, j)) = undo_stack.last_mut() {
                if let Some(next_split) = split.pop() {
                    grid.clone_from(base_grid);
                    heads.clone_from(base_heads);

                    grid[next_split] = *col;
                    if *j == 0 {
                        heads[*i].0 = next_split;
                    } else {
                        heads[*i].1 = next_split;
                    }
                    trace!("Reverting grid and trying next");
                } else {
                    trace!("No guesses made were correct - undoing guess");
                    undo_stack.pop();
                }
            } else {
                trace!("Early exit - base grid impossible");
                return (grid, false);
            }
        } else if !progress_made {
            trace!("No progress made");
            let (mut split, col, i, j) = guessing_target.unwrap(); // should never fail...
            trace!("Options are: {:?}", split);
            let next_step = split.pop().unwrap();

            assert!(split.len() != 0);
            undo_stack.push((grid.clone(), heads.clone(), split, col, i, j));

            grid[next_step] = col;
            if j == 0 {
                heads[i].0 = next_step;
            } else {
                heads[i].1 = next_step;
            }
        }

        trace!("Grid:\n{grid}");
    }
}

#[test]
fn test() {
    let heads = vec![
        ((0, 1), (1, 3)),
        ((1, 1), (3, 1)),
        ((0, 2), (4, 4)),
        ((2, 3), (4, 3)),
    ];
    let mut grid = Array2D::new(5, 5);
    for (h, (a, b)) in heads.iter().enumerate() {
        grid[*a] = h as u8 + 1;
        grid[*b] = h as u8 + 1;
    }

    let grid = solve(grid, heads).0;

    assert!(!grid.contains_zeroes());
}

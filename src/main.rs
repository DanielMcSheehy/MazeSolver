use druid::piet::Color;
use druid::widget::{Button, Flex, Painter};
use druid::Data;
use druid::RenderContext;
use druid::{AppLauncher, PlatformError, Widget, WidgetExt, WindowDesc};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

mod traverse;

const DEFAULT_HEIGHT: i32 = 9;
const DEFAULT_WIDTH: i32 = 10;

const DEFAULT_START_X: i32 = 1;
const DEFAULT_START_Y: i32 = 5;

const DEFAULT_END_X: i32 = 8;
const DEFAULT_END_Y: i32 = 5;

#[derive(Clone)]
enum ButtonState {
    NewGame,
    Obstacle,
    Start,
}

#[derive(Clone, PartialEq, Debug)]
pub enum SquareKind {
    Init,
    Obstacle,
    PossiblePath,
    SolutionPath,
    StartSquare,
    EndSquare,
}

#[derive(Clone, Debug)]
pub struct Node {
    x: i32,
    y: i32,

    parent: Option<Rc<Node>>,
}

impl Node {
    pub fn new(x: i32, y: i32) -> Node {
        Node {
            x: x,
            y: y,
            parent: None,
        }
    }

    pub fn add(&self, child_node: &mut Node) {
        child_node.parent = Some(Rc::new(self.clone()));
    }

    pub fn find_reverse_path(&self) -> Vec<(i32, i32)> {
        let mut result: Vec<(i32, i32)> = vec![];
        let mut current_node = &Rc::new(self.clone());

        while !current_node.parent.is_none() {
            result.push((current_node.x, current_node.y));
            current_node = current_node.parent.as_ref().unwrap();
        }

        result
    }
}

#[derive(Clone, Debug)]
// (x, y, previous position)
struct Move(i32, i32, Node);

trait Metadata {
    fn new() -> Self;
    fn gen_board(&self, height: i32, width: i32) -> Flex<State>;
    fn init_state(height: i32, width: i32) -> Arc<Vec<Vec<SquareKind>>>;
    fn clear(&mut self);
    // traversal methods
    fn is_valid_move(&self, x: i32, y: i32) -> bool;
    fn get_possible_moves(&self, parent: Node) -> Vec<Move>;
    fn traverse(&mut self) -> Vec<(i32, i32)>;
}

#[derive(Clone, Data)]
struct State {
    button_state: Arc<ButtonState>,
    width: i32,
    height: i32,
    solved: bool,
    state: Arc<Vec<Vec<SquareKind>>>,
}

impl Metadata for State {
    fn init_state(height: i32, width: i32) -> Arc<Vec<Vec<SquareKind>>> {
        let mut state: Vec<Vec<SquareKind>> = vec![vec![]];

        for column in 0..height {
            state.push(vec![]);
            for row in 0..width {
                let mut square_type = SquareKind::Init;
                if column == DEFAULT_START_Y && row == DEFAULT_START_X {
                    square_type = SquareKind::StartSquare;
                } else if column == DEFAULT_END_Y && row == DEFAULT_END_X {
                    square_type = SquareKind::EndSquare;
                }
                state[column as usize].push(square_type)
            }
        }

        Arc::new(state)
    }
    fn gen_board(&self, height: i32, width: i32) -> Flex<State> {
        let mut board = Flex::column();
        for y in 0..height {
            board = board.with_flex_child(gen_square_row(y, width), 1.0);
        }
        board
    }
    fn clear(&mut self) {
        self.button_state = Arc::new(ButtonState::NewGame);
        self.state = State::init_state(self.height, self.width);
    }
    fn new() -> Self {
        State {
            button_state: Arc::new(ButtonState::NewGame),
            height: DEFAULT_HEIGHT,
            width: DEFAULT_WIDTH,
            solved: false,
            state: Self::init_state(DEFAULT_HEIGHT, DEFAULT_WIDTH),
        }
    }
    fn is_valid_move(&self, x: i32, y: i32) -> bool {
        // check if move is out of bounds
        if y < 0 || y >= self.height || x < 0 || x >= self.width {
            return false;
        }

        // check if move is on an obstacle, or the start square,
        // allow all other kinds.
        let square_kind = &self.state[y as usize][x as usize];

        return match square_kind {
            SquareKind::Obstacle => false,
            SquareKind::StartSquare => false,
            SquareKind::Init => true,
            SquareKind::SolutionPath => true,
            SquareKind::PossiblePath => true,
            SquareKind::EndSquare => true,
        };
    }
    fn get_possible_moves(&self, parent: Node) -> Vec<Move> {
        let cur_x = parent.x;
        let cur_y = parent.y;
        let mut result: Vec<Move> = vec![];
        let mut all_moves = vec![
            Move(cur_x + 1, cur_y, parent.clone()),
            Move(cur_x - 1, cur_y, parent.clone()),
            Move(cur_x, cur_y - 1, parent.clone()),
            Move(cur_x, cur_y + 1, parent.clone()),
        ];
        all_moves.reverse();

        for m in all_moves {
            if self.is_valid_move(m.0, m.1) {
                result.push(m)
            }
        }

        result
    }
    fn traverse(&mut self) -> Vec<(i32, i32)> {
        let mut visited = HashMap::new();

        let mut stack: Vec<Move> =
            self.get_possible_moves(Node::new(DEFAULT_START_X, DEFAULT_START_Y));

        while stack.len() > 0 {
            let m = stack.pop().unwrap();
            let (cur_x, cur_y) = (m.0, m.1);
            let parent = &m.2.clone();
            let mut child = Node::new(cur_x, cur_y);
            parent.add(&mut child);

            if self.state[cur_y as usize][cur_x as usize] == SquareKind::EndSquare {
                self.solved = true;

                for square in parent.find_reverse_path() {
                    let (x, y) = square;
                    Arc::make_mut(&mut self.state)[y as usize][x as usize] =
                        SquareKind::SolutionPath;
                }

                return parent.find_reverse_path();
            }

            let value = format!("{},{}", cur_x, cur_y);

            if *visited.get(&value).unwrap_or(&false) {
                continue;
            }

            visited.insert(value, true);

            Arc::make_mut(&mut self.state)[cur_y as usize][cur_x as usize] =
                SquareKind::PossiblePath;

            for m in self.get_possible_moves(child) {
                stack.push(m)
            }
        }

        vec![]
    }
}

fn square(y: i32, x: i32) -> impl Widget<State> {
    Painter::new(move |ctx, data: &State, _| {
        let bounds = ctx.size().to_rect();

        let color = match data.state[y as usize][x as usize] {
            SquareKind::Init => &Color::WHITE,
            SquareKind::Obstacle => &Color::BLACK,
            SquareKind::PossiblePath => &Color::WHITE,
            SquareKind::SolutionPath => &Color::YELLOW,
            SquareKind::StartSquare => &Color::GREEN,
            SquareKind::EndSquare => &Color::PURPLE,
        };
        ctx.fill(bounds, color);
        ctx.stroke(bounds.inset(-0.5), &Color::BLACK, 1.0);
    })
    .on_click(move |_ctx, data: &mut State, _env| {
        Arc::make_mut(&mut data.state)[y as usize][x as usize] = match *data.button_state {
            ButtonState::Obstacle => SquareKind::Obstacle,
            ButtonState::NewGame => SquareKind::Init,
            ButtonState::Start => SquareKind::Init,
        };
    })
}

fn gen_square_row(y: i32, width: i32) -> impl Widget<State> {
    let mut row = Flex::row();
    for x in 0..width {
        row = row.with_flex_child(square(y, x), 1.0);
    }
    row
}

fn main() -> Result<(), PlatformError> {
    let data = State::new();
    let main_window = WindowDesc::new(ui_builder(&data));
    AppLauncher::with_window(main_window)
        .log_to_console()
        .launch(data)
}

fn ui_builder(data: &State) -> impl Widget<State> {
    let start_button = Button::new("start")
        .on_click(|_ctx, data: &mut State, _env| {
            *Arc::make_mut(&mut data.button_state) = ButtonState::Start;
            data.traverse();
        })
        .padding(5.0);

    let obstacle_button = Button::new("add obstacles")
        .on_click(|_ctx, data: &mut State, _env| {
            *Arc::make_mut(&mut data.button_state) = ButtonState::Obstacle;
        })
        .padding(5.0);

    let new_game_button = Button::new("new game")
        .on_click(|_ctx, data: &mut State, _env| data.clear())
        .padding(5.0);

    data.gen_board(DEFAULT_HEIGHT, DEFAULT_WIDTH)
        .with_flex_spacer(2.0)
        .with_child(start_button)
        .with_child(obstacle_button)
        .with_child(new_game_button)
}

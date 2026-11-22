pub mod picker;
pub mod session;

use picker::PickerScreen;
use session::SessionScreen;

pub enum Screen {
    Picker(PickerScreen),
    Session(Box<SessionScreen>),
}

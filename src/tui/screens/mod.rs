pub mod home;
pub mod picker;
pub mod session;

use home::HomeScreen;
use picker::PickerScreen;
use session::SessionScreen;

pub enum Screen {
    Home(HomeScreen),
    Picker(PickerScreen),
    Session(SessionScreen),
}

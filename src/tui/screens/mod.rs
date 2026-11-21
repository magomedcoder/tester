pub mod home;
pub mod picker;

use home::HomeScreen;
use picker::PickerScreen;

pub enum Screen {
    Home(HomeScreen),
    Picker(PickerScreen),
}

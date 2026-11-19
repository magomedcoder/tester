pub mod home;

use home::HomeScreen;

pub enum Screen {
    Home(HomeScreen),
}

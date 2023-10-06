use core::fmt::Debug;

use embedded_hal_async::i2c::I2c;
use ssd1306::Ssd1306;

pub trait Ssd1306I2cDisplayDriver {
    fn setup_i2c() -> impl I2c<Error = impl Debug>;
}

pub async fn setup_display_driver() -> Ssd1306<> {
    let test = Ssd1306()
}

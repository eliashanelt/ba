use pressure_api::PressureAPI;

mod pressure_api;

const PRESSURE_API_PATH: &str = "C://Program Files/Cytosurge/CORA/Cytosurge.PressureAPI.dll";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    let pressure_controller = PressureAPI::new(PRESSURE_API_PATH)?;
    pressure_controller.connect(conn_error_callback)?;

    let (min_val, max_val) = pressure_controller.read_pressure_range()?;
    println!("Allowed range: [{:.2}; {:.2}] mbar", min_val, max_val);

    pressure_controller.set_pressure(max_val)?;

    let current_pressure = pressure_controller.get_pressure()?;
    println!("Current pressure: {:.2} mbar", current_pressure);

    Ok(())
}

unsafe extern "system" fn conn_error_callback(msg: *const i8) {
    if !msg.is_null() {
        let c_str = std::ffi::CStr::from_ptr(msg);
        println!("The device experienced a runtime error: {:?}", c_str);
    }
}

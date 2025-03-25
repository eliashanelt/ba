use libloading::{Library, Symbol};
use std::ffi::c_void;

// Define a type to mirror your "opaque" handle from the C API.
#[repr(C)]
pub struct PressureAPIHandle {
    _private: [u8; 0], // Not meant to be used directly in Rust.
}

// Callback type used for error reporting (as an example).
type ErrorCallback = unsafe extern "system" fn(*const i8);

// A simple struct that holds both the loaded library and the handle to your API.
pub struct PressureAPI {
    lib: Library,                // The dynamic library handle
    api: *mut PressureAPIHandle, // The "native" API handle
}

// For convenience, define function pointer types that match your C API signatures:
type PressureAPICreateFn = unsafe extern "system" fn() -> *mut PressureAPIHandle;
type PressureAPIDestroyFn = unsafe extern "system" fn(*mut PressureAPIHandle);
type PressureAPIConnectFn = unsafe extern "system" fn(*mut PressureAPIHandle, ErrorCallback);
type PressureAPIReadRangeFn = unsafe extern "system" fn(*mut PressureAPIHandle, *mut f64, *mut f64);
type PressureAPISetPressureFn = unsafe extern "system" fn(*mut PressureAPIHandle, f64);
type PressureAPIGetPressureFn = unsafe extern "system" fn(*mut PressureAPIHandle) -> f64;

impl PressureAPI {
    /// Load the DLL and create a PressureAPI instance.
    pub fn new(dll_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Open the library
        let lib = unsafe { Library::new(dll_path)? };

        // 2. Load the "create" symbol
        let create_fn: Symbol<PressureAPICreateFn> = unsafe { lib.get(b"PressureAPI_Create")? };

        // 3. Call it to get the native handle
        let api = unsafe { create_fn() };
        if api.is_null() {
            return Err("Failed to create PressureAPI handle".into());
        }

        // Return a new PressureAPI struct which holds the library and handle
        Ok(PressureAPI { lib, api })
    }

    /// Connect to the device and provide an error callback.
    pub fn connect(&self, callback: ErrorCallback) -> Result<(), Box<dyn std::error::Error>> {
        let connect_fn: Symbol<PressureAPIConnectFn> =
            unsafe { self.lib.get(b"PressureAPI_Connect")? };
        unsafe {
            connect_fn(self.api, callback);
        }
        Ok(())
    }

    /// Read the supported pressure range.
    pub fn read_pressure_range(&self) -> Result<(f64, f64), Box<dyn std::error::Error>> {
        let read_range_fn: Symbol<PressureAPIReadRangeFn> =
            unsafe { self.lib.get(b"PressureAPI_ReadPressureRange")? };

        let mut min_val = 0.0;
        let mut max_val = 0.0;
        unsafe {
            read_range_fn(self.api, &mut min_val, &mut max_val);
        }
        Ok((min_val, max_val))
    }

    /// Set a pressure setpoint.
    pub fn set_pressure(&self, pressure: f64) -> Result<(), Box<dyn std::error::Error>> {
        let set_pressure_fn: Symbol<PressureAPISetPressureFn> =
            unsafe { self.lib.get(b"PressureAPI_SetPressure")? };
        unsafe {
            set_pressure_fn(self.api, pressure);
        }
        Ok(())
    }

    /// Get the currently active pressure.
    pub fn get_pressure(&self) -> Result<f64, Box<dyn std::error::Error>> {
        let get_pressure_fn: Symbol<PressureAPIGetPressureFn> =
            unsafe { self.lib.get(b"PressureAPI_GetPressure")? };
        let pressure = unsafe { get_pressure_fn(self.api) };
        Ok(pressure)
    }
}

// Ensure we clean up via the library’s Destroy function.
impl Drop for PressureAPI {
    fn drop(&mut self) {
        unsafe {
            // Attempt to load the destroy symbol
            if let Ok(destroy_fn) = self
                .lib
                .get::<Symbol<PressureAPIDestroyFn>>(b"PressureAPI_Destroy")
            {
                destroy_fn(self.api);
            }
        }
    }
}

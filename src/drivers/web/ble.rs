use crate::drivers::ble::{
    get_uuid, get_uuid_from_name, Characteristic, Server, Service, UUIDAble,
};
use anyhow::Result;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    fn register_ble_service_js(uuid: &str);
    fn register_ble_characteristic_js(uuid: &str, is_read: bool, is_write: bool);
    fn start_ble_advertisement_js();
    fn send_characteristic_value_js(char_id: &str, value: &[u8]);
}

type CallbackFn = dyn Fn(&[u8]) -> Result<()> + 'static;

thread_local! {
    static CHARACTERISTICS: RefCell<HashMap<String, Rc<RefCell<Box<CallbackFn>>>>> = RefCell::new(HashMap::new());
}

#[wasm_bindgen]
pub fn on_characteristic_write(char_id: &str, data: &[u8]) -> bool {
    let mut success = false;
    CHARACTERISTICS.with(|chars| {
        if let Some(callback_rc) = chars.borrow().get(char_id) {
            let callback = callback_rc.borrow();
            success = callback(data).is_ok();
        }
    });
    success
}

pub struct BLECharacteristicSimImpl {
    name: String,
    is_read: bool,
    is_write: bool,
}

impl UUIDAble for BLECharacteristicSimImpl {
    fn get_name(&self) -> &str {
        &self.name
    }
}

impl Characteristic for BLECharacteristicSimImpl {
    fn set_callback(&self, callback: impl Fn(&[u8]) -> Result<()> + Send + Sync + 'static) {
        // Register callback in the global registry
        CHARACTERISTICS.with(|chars| {
            chars.borrow_mut().insert(
                get_uuid(self).to_string(),
                Rc::new(RefCell::new(Box::new(callback))),
            );
        });
    }

    fn send_value<'a>(&self, value: &'a [u8]) {
        send_characteristic_value_js(&get_uuid(self).to_string(), value);
    }
}

pub struct BLEServiceSimImpl {
    name: String,
    characteristics: RefCell<HashMap<String, BLECharacteristicSimImpl>>,
}

impl UUIDAble for BLEServiceSimImpl {
    fn get_name(&self) -> &str {
        &self.name
    }
}

impl Service for BLEServiceSimImpl {
    type Characteristic = BLECharacteristicSimImpl;

    fn register_characteristic(
        &mut self,
        name: &str,
        is_read: bool,
        is_write: bool,
    ) -> Result<Self::Characteristic> {
        let uuid = get_uuid_from_name(name).to_string();
        register_ble_characteristic_js(&uuid, is_read, is_write);

        let characteristic = BLECharacteristicSimImpl {
            name: name.to_string(),
            is_read,
            is_write,
        };

        self.characteristics
            .borrow_mut()
            .insert(uuid, characteristic.clone());
        Ok(characteristic)
    }
}

impl Clone for BLECharacteristicSimImpl {
    fn clone(&self) -> Self {
        BLECharacteristicSimImpl {
            name: self.name.clone(),
            is_read: self.is_read,
            is_write: self.is_write,
        }
    }
}

pub struct BLEServerSimImpl {
    services: RefCell<HashMap<String, BLEServiceSimImpl>>,
}

impl Server for BLEServerSimImpl {
    type Service = BLEServiceSimImpl;

    fn new() -> Self {
        BLEServerSimImpl {
            services: RefCell::new(HashMap::new()),
        }
    }

    fn register_service(&mut self, name: &str) -> Result<Self::Service> {
        let uuid = get_uuid_from_name(name).to_string();
        register_ble_service_js(&uuid);

        let service = BLEServiceSimImpl {
            name: name.to_string(),
            characteristics: RefCell::new(HashMap::new()),
        };

        self.services.borrow_mut().insert(uuid, service.clone());
        Ok(service)
    }

    fn start_advertisement(&mut self) {
        start_ble_advertisement_js();
    }
}

impl Clone for BLEServiceSimImpl {
    fn clone(&self) -> Self {
        BLEServiceSimImpl {
            name: self.name.clone(),
            characteristics: RefCell::new(self.characteristics.borrow().clone()),
        }
    }
}

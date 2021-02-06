mod instance;
use ash::version::EntryV1_0;
use instance::Instance;

use anyhow::Result;
use ash::vk;

use std::ffi::CString;
use std::sync::Arc;

pub struct Entry {
    handle: ash::Entry,
    instance: Option<Instance>,
}

impl Entry {
    pub fn new() -> Result<Self> {
        let handle = ash::Entry::new()?;

        let result = Self {
            handle,
            instance: None,
        };

        Ok(result)
    }

    pub fn vulkan_version(&self) -> String {
        let version_str = match self.handle.try_enumerate_instance_version().unwrap() {
            // Vulkan 1.1+
            Some(version) => {
                let major = vk::version_major(version);
                let minor = vk::version_minor(version);
                let patch = vk::version_patch(version);
                format!("{}.{}.{}", major, minor, patch)
            }
            // Vulkan 1.0
            None => String::from("1.0"),
        };
        version_str
    }

    pub fn create_instance(
        &mut self,
        layer_names: &[&str],
        extension_names: &[&str],
    ) -> Arc<Instance> {
        let app_name = CString::new(env!("CARGO_PKG_NAME")).unwrap();
        let engine_name = CString::new("Silly Cat Engine").unwrap();

        let appinfo = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(0)
            .engine_name(&engine_name)
            .engine_version(0)
            .api_version(vk::make_version(1, 2, 0));

        let layer_names = layer_names
            .iter()
            .map(|s| CString::new(*s).unwrap())
            .collect::<Vec<_>>();
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let extension_names = extension_names
            .iter()
            .map(|s| CString::new(*s).unwrap())
            .collect::<Vec<_>>();
        let mut extension_names_raw = extension_names
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&appinfo)
            .enabled_layer_names(&layers_names_raw)
            .enabled_extension_names(&extension_names_raw);

        self.instance =
            Some(unsafe { self.handle.create_instance(vk::Instancecreateinfo).unwrap() });
    }
}

#[cfg(test)]
mod tests {
    use super::Entry;

    #[test]
    fn test_entry() {
        let entry = Entry::new().unwrap();
        println!("Vulkan version {}", entry.vulkan_version());
    }
}

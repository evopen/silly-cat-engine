mod instance;
use instance::Instance;

use anyhow::Result;
use ash::vk;

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

    pub fn create_instance(&mut self) {}
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

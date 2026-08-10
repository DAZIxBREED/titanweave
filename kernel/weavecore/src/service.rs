#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceRole {
    Init,
    Logging,
    Console,
    Display,
    Shell,
    Archive,
    Trust,
    DriverHost,
    Audio,
}

pub struct ServiceSpec {
    pub path: &'static [u8],
    pub process_name: &'static [u8],
    pub role: ServiceRole,
}

pub const SERVICE_SPECS: [ServiceSpec; 9] = [
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\INIT.ELF", process_name: b"init", role: ServiceRole::Init },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\LOGD.ELF", process_name: b"logging", role: ServiceRole::Logging },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\CONSOL.ELF", process_name: b"console", role: ServiceRole::Console },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\DISPLAYD.ELF", process_name: b"displayd", role: ServiceRole::Display },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\ARCHIVE.ELF", process_name: b"archive", role: ServiceRole::Archive },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\TRUSTD.ELF", process_name: b"trust", role: ServiceRole::Trust },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\DRIVERD.ELF", process_name: b"driver-host", role: ServiceRole::DriverHost },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\AUDIOD.ELF", process_name: b"forgeaudiod", role: ServiceRole::Audio },
    ServiceSpec { path: b"C:\\SYSTEM\\SERVICES\\SHELL.ELF", process_name: b"shell", role: ServiceRole::Shell },
];

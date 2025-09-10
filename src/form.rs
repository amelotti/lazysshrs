#[derive(Debug, Clone)]
pub struct HostForm {
    pub folder: String,
    pub host: String,
    pub hostname: String,
    pub user: String,
    pub port: String,
    pub identity_file: String,
    pub local_forward: String,
    pub current_field: usize,
}

impl Default for HostForm {
    fn default() -> Self {
        Self {
            folder: String::new(),
            host: String::new(),
            hostname: String::new(),
            user: String::new(),
            port: String::new(),
            identity_file: String::new(),
            local_forward: String::new(),
            current_field: 0,
        }
    }
}

impl HostForm {
    pub fn field_names() -> Vec<&'static str> {
        vec!["Pasta", "Host", "Hostname", "User", "Port", "IdentityFile", "LocalForward"]
    }

    pub fn get_field(&self, index: usize) -> &str {
        match index {
            0 => &self.folder,
            1 => &self.host,
            2 => &self.hostname,
            3 => &self.user,
            4 => &self.port,
            5 => &self.identity_file,
            6 => &self.local_forward,
            _ => "",
        }
    }

    pub fn set_field(&mut self, index: usize, value: String) {
        match index {
            0 => self.folder = value,
            1 => self.host = value,
            2 => self.hostname = value,
            3 => self.user = value,
            4 => self.port = value,
            5 => self.identity_file = value,
            6 => self.local_forward = value,
            _ => {}
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.folder.is_empty() && !self.host.is_empty() && !self.hostname.is_empty() && !self.user.is_empty()
    }

    pub fn next_field(&mut self) {
        self.current_field = (self.current_field + 1) % 7;
    }

    pub fn prev_field(&mut self) {
        self.current_field = if self.current_field == 0 { 6 } else { self.current_field - 1 };
    }
}
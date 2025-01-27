use local_ip_addr::get_local_ip_address;

pub fn get_local_socket_address() -> Option<std::net::SocketAddr> {
    match get_local_ip_address() {
        Ok(ip) => {
            let ip_socket_local = ip + ":7878";
            match ip_socket_local.parse() {
                Ok(ip_socket_local) => Some(ip_socket_local),
                Err(e) => {
                    None
                }
            }
        },
        Err(e) => {
            None
        }
    }
}
pub fn get_udp_socket_address() -> Option<std::net::SocketAddr> {
    match get_local_ip_address() {
        Ok(ip) => {
            let ip_socket_local = ip + ":8000";
            match ip_socket_local.parse() {
                Ok(ip_socket_local) => Some(ip_socket_local),
                Err(e) => {
                    None
                }
            }
        },
        Err(e) => {
            None
        }
    }
}

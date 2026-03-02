use serde::Serialize;

pub fn print_json<T: Serialize>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{}", json),
        Err(e) => print_error(&format!("Failed to serialize output: {}", e)),
    }
}

pub fn print_error(msg: &str) {
    let error = serde_json::json!({"error": msg});
    println!(
        "{}",
        serde_json::to_string_pretty(&error)
            .unwrap_or_else(|_| format!("{{\"error\": {:?}}}", msg))
    );
}

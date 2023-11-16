use wayforge::init_wayforge;

fn main() {
    // logging 
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }

    tracing::info!("Starting wayforge...");
    init_wayforge().expect("Could not initialize wayforge");
}

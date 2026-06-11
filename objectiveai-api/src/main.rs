use envconfig::Envconfig;

#[tokio::main]
async fn main() {
    let _ = dotenv::dotenv();
    let config = objectiveai_api::ConfigBuilder::init_from_env().unwrap().build();
    objectiveai_api::run(config).await.unwrap();
}

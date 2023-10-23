pub mod release {
    use log::error;

    type DynError = Box<dyn std::error::Error>;

    pub async fn release() -> Result<(), DynError> {
        if let Err(error) = try_release().await {
            error!("{error:#}");

            std::process::exit(-1);
        }

        Ok(())
    }

    async fn try_release() -> Result<(), DynError> {
        todo!("release")
    }
}

use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;

struct Storage {
    region: Region,
    credentials: Credentials,
    bucket: String,
}

pub async fn download_layers_s3() -> Result<(), S3Error> {
    let aws = Storage {
        region: "us-west-1".parse()?,
        credentials: Credentials::from_env_specific(
            Some("AWS_ACCESS_KEY_ID"),
            Some("AWS_SECRET_ACCESS_KEY"),
            None,
            None,
        )?,
        bucket: "orbi-bot".to_string(),
    };

    let bucket = Bucket::new(&aws.bucket, aws.region, aws.credentials)?;

    let results = bucket.list("".to_string(), None).await?;
    for result in results {
        for object in result.contents {
            log::debug!("Downloading {}", object.key);

            let dir_path = std::path::Path::new(&object.key).parent().unwrap();
            tokio::fs::create_dir_all(dir_path).await?;
            let mut outfile = tokio::fs::File::create(&object.key).await?;

            let response_code = bucket.get_object_stream(&object.key, &mut outfile).await?;
            assert_eq!(200, response_code);
        }
    }

    Ok(())
}

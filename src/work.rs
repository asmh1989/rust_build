use crate::build_params::*;

pub fn getSource(params: BuildParams) {
    let path = "/home/sun/build_cache";
    let url = params.version.source_url.as_str();

    if Scm::Git == params.version.scm {}
}

pub fn start(_params: BuildParams) {}

#[cfg(test)]

mod tests {
    use super::{getSource, BuildParams, Framework, Scm};
    use serde_json::Result;

    fn http_params() -> Result<BuildParams> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let data = r#"
        {
            "version" : {
                "scm" : "git",
                "source_url" : "https://github.com/asmh1989/okhttp4_demo.git"
            },
            "configs" : {
                "framework": "normal"
            }
        }"#;

        // Parse the string of data into a Person object. This is exactly the
        // same function as the one that produced serde_json::Value above, but
        // now we are asking it for a Person as output.
        let p: BuildParams = serde_json::from_str(data)?;

        // Do things just like with any other Rust data structure.
        // println!("build params =  {:?}", p);

        // println!(
        //     "build params =  {}",
        //     serde_json::to_string_pretty(&p).ok().unwrap()
        // );

        Ok(p)
    }
    #[test]
    fn test_http_clone() {
        let result = http_params();

        let params = result.unwrap();

        getSource(params);

        assert!(true);
    }
}

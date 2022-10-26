use std::collections::HashMap;
use std::process::Command;
use std::{fs, path::Path};

use clap::Parser;
use reqwest::header::{HeaderMap, HeaderValue};

use serde_json::Value;

// const DEFAULT_WORKSPACE: &'static str = "/opt/huawei/release/hispace/AppGalleryQuickAppService/searchtest/fastapp/startres/application/webquickapp";
const DEFAULT_WORKSPACE: &'static str = "/opt/xxxx/yyyy";
const TMP_DOWNLOAD_DIR: &'static str = "/tmp/download";

#[derive(Parser, Debug)]
struct CmdArgs {
    #[arg(short, long)]
    package_name: String,
    #[arg(short, long)]
    workspace: Option<String>,
    #[arg(short, long)]
    replace_if_exists: Option<bool>,
}

#[derive(Debug)]
struct DeployParam {
    package_name: String,
    deploy_dir: String,
    replace_if_exists: bool,
}

impl From<CmdArgs> for DeployParam {
    fn from(cmd_args: CmdArgs) -> Self {
        let workspace = cmd_args.workspace.unwrap_or(DEFAULT_WORKSPACE.into());
        let package_name = cmd_args.package_name;
        DeployParam {
            package_name: package_name.clone(),
            deploy_dir: format!("{}/{}", workspace, package_name),
            replace_if_exists: cmd_args.replace_if_exists.unwrap_or(true),
        }
    }
}

/// download start.
type DownloadErrMsg = String;
type DownloadResult = String;

trait RpkDownload {
    fn download(
        &self,
        package_name: String,
        download_url: String,
    ) -> Result<DownloadResult, DownloadErrMsg>;
}

struct DefaultDownloader;

impl RpkDownload for DefaultDownloader {
    fn download(
        &self,
        package_name: String,
        download_url: String,
    ) -> Result<DownloadResult, DownloadErrMsg> {
        let tmp_download_path = format!("{}/{}.rpk", TMP_DOWNLOAD_DIR, package_name);
        println!("saved rpk file to: {}", tmp_download_path);
        let resp_wrapper =
            reqwest::blocking::get(download_url.as_str()).and_then(|resp| resp.bytes());
        if let Err(err) = resp_wrapper {
            return Err(err.to_string());
        }
        let bytes = resp_wrapper.unwrap();
        let write_res = fs::write(&tmp_download_path, bytes.to_vec().as_slice());
        match write_res {
            Ok(_) => Ok(tmp_download_path),
            Err(err) => Err(err.to_string()),
        }
    }
}

/// download end.

/// unzip start.
trait RpkUnZip {
    fn unzip(zip_path: &str, unzip_path: &str) -> Result<(), String>;
}

struct ShellUnzip;

impl RpkUnZip for ShellUnzip {
    fn unzip(zip_path: &str, unzip_path: &str) -> Result<(), String> {
        let unzip_cmd = format!("unzip {} -d {}", zip_path, unzip_path);
        println!("unzip cmd is: {}", unzip_cmd);
        let exec_res = Command::new("unzip")
            .arg(zip_path)
            .arg("-d")
            .arg(unzip_path)
            .output();
        exec_res.map(|_| ()).map_err(|err| err.to_string())
    }
}

fn search_rpk_by_packagename(package_name: &str) -> Result<String, String> {
    let payload_template = include_str!("req_payload.txt");
    let payload = format!("{}{}", payload_template, package_name);
    println!("payload: {}", payload);
    let payload = payload.as_bytes().to_vec();
    reqwest::blocking::ClientBuilder::new()
        .gzip(true)
        .build()
        .and_then(move |client| {
            let mut headers = HeaderMap::new();
            headers.insert(
                reqwest::header::CONTENT_TYPE,
                HeaderValue::from_static("text/plain"),
            );
            client
                .post("http://stores1.hispace.hicloud.com/hwmarket/api/tlsApis")
                .headers(headers)
                .body(payload)
                .send()
        })
        .and_then(|resp| resp.json())
        .map(|body: HashMap<String, Value>| {
            println!("body: {:#?}", body);
            let dld_url_wrapper = body
                .get("rpkInfo")
                .and_then(|rpk| rpk.get("url"))
                .map(|dld_url| dld_url.as_str().unwrap());
            // body
            match dld_url_wrapper {
                None => String::new(),
                Some(dld_url) => dld_url.into(),
            }
        })
        .map_err(|err| err.to_string())
}

struct DeployProcessor(DeployParam);

impl DeployProcessor {
    fn process(&self) -> Result<(), String> {
        // init exec environment.
        self.env_init();
        let param = &self.0;
        let package_name = &param.package_name;
        let deploy_dir = &param.deploy_dir;
        let downloader = DefaultDownloader;

        search_rpk_by_packagename(package_name)
            .and_then(|download_url| downloader.download(package_name.clone(), download_url))
            .and_then(|dld_res| ShellUnzip::unzip(&dld_res, &deploy_dir))
            .and_then(|_| DeployProcessor::create_cert_sign(&deploy_dir))
    }

    fn create_cert_sign(unzip_path: &str) -> Result<(), String> {
        fs::write(&format!("{}/cp.cert", unzip_path), "123123123").map_err(|err| err.to_string())
    }

    fn env_init(&self) {
        let download_dir = Path::new(TMP_DOWNLOAD_DIR);
        if !download_dir.exists() {
            let _ = fs::create_dir_all(download_dir);
        }
        let target_dep_dir = Path::new(&self.0.deploy_dir);
        if !target_dep_dir.exists() {
            let _ = fs::create_dir_all(target_dep_dir);
        } else if self.0.replace_if_exists {
            fs::remove_dir_all(target_dep_dir).expect("failed to delete old version");
            println!("removed old rpk... {:?}", &target_dep_dir);
            let _ = fs::create_dir_all(target_dep_dir);
        }
    }
}

/// unzip end.
fn main() {
    let cmd_args = CmdArgs::parse();
    let deploy_param: DeployParam = DeployParam::from(cmd_args);
    let package_name = &deploy_param.package_name.clone();
    println!("runtime params: {:#?}", &deploy_param);
    println!("workspace: {}", &deploy_param.deploy_dir);
    match DeployProcessor(deploy_param).process() {
        Ok(_) => {
            println!("succeed to deploy webquickapp {} in dev env.", package_name)
        }
        Err(err) => {
            eprintln!("failed to deploy webquickapp {}: {:?}", package_name, err)
        }
    };
}

#[cfg(test)]
mod testsuite {
    #[test]
    fn test_cmd_args_parse() {}
}

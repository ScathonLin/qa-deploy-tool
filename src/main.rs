use std::{fs, fs::File, path::Path};

use clap::Parser;
use zip::ZipArchive;

const DEFAULT_WORKSPACE: &'static str = "/tmp/dep-tools/workspace";
const DEFAULT_RPK_DOWNLOAD_URL: &'static str =
    "http://localhost:8080/io.scathon.quickapp.helloworld.rpk";
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
    fn download(&self, package_name: String) -> Result<DownloadResult, DownloadErrMsg>;
}

struct DefaultDownloader;

impl RpkDownload for DefaultDownloader {
    fn download(&self, package_name: String) -> Result<DownloadResult, DownloadErrMsg> {
        let tmp_download_path = format!("{}/{}.rpk", TMP_DOWNLOAD_DIR, package_name);
        println!("saved rpk file to: {}", tmp_download_path);
        let resp_wrapper =
            reqwest::blocking::get(DEFAULT_RPK_DOWNLOAD_URL).and_then(|resp| resp.bytes());
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

struct DefaultRpkUnzip;

impl RpkUnZip for DefaultRpkUnzip {
    fn unzip(zip_path: &str, unzip_path: &str) -> Result<(), String> {
        // 这里文件肯定存在，所以直接unwrap
        let zip_file = File::open(zip_path).unwrap();
        ZipArchive::new(zip_file)
            .and_then(|mut zip_file| zip_file.extract(unzip_path))
            .map_err(|res| res.to_string())
    }
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

        downloader
            .download(package_name.clone())
            .and_then(|dld_res| DefaultRpkUnzip::unzip(&dld_res, &deploy_dir))
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

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

extern crate yaml_rust;
extern crate pcre2;

use yaml_rust::{YamlLoader, YamlEmitter};
use pcre2::bytes::{Regex};

use std::env;
use std::io::prelude::*;
use yaml_rust::yaml;

use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use	std::cmp::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::thread;
use chrono::prelude::*;
use std::str;
use serde_json::json;
use serde_json::value::Value;

use amiquip::{
    Connection, ConsumerMessage, ConsumerOptions, Exchange, ExchangeDeclareOptions, ExchangeType,
    FieldTable, Publish, QueueDeclareOptions, Result,
};

#[derive(Deserialize, Debug, Serialize)]
struct Code {
    code: Vec<String>,
    // location: String,
    // spArr: Vec<String>,
}
// #[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Deserialize, Debug, Serialize)]
struct Realdata {
    code: String,
    name: String,
    open: f64,
    close: f64,
    now: f64,
    price: f64,
    high: f64,
    low: f64,
    buy: f64,
    sell: f64,
    turnover: i64,
    amount: i64,
    volume: f64,
    bid1_volume: i64,
    bid1: f64,
    bid2_volume: i64,
    bid2: f64,
    bid3_volume: i64,
    bid3: f64,
    bid4_volume: i64,
    bid4: f64,
    bid5_volume: i64,
    bid5: f64,
    ask1_volume: i64,
    ask1: f64,
    ask2_volume: i64,
    ask2: f64,
    ask3_volume: i64,
    ask3: f64,
    ask4_volume: i64,
    ask4: f64,
    ask5_volume: i64,
    ask5: f64,
    date: String,
    time: String,
    datetime: String,
}

impl Realdata {
    fn to_json(&mut self) -> String {
        let jdata = serde_json::to_string(&self).unwrap();
        //println!("this is json{:#?}", jdata);
        jdata
    }
}
struct SplitCode {
    codes: Vec<String>,
}

fn read_user_from_file<P: AsRef<Path>>(path: P) -> Result<Code, Box<Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u = serde_json::from_reader(reader)?;

    // Return the `Code`.
    // u.spArr = Vec::new();
    Ok(u)
}

// extern crate qadata_rs;
// use qadata_rs::qafetch::QAMongoClient;

// #[tokio::main]
// fn main() -> Result<(), reqwest::Error> {
fn main() {
    let args: Vec<_> = env::args().collect();
    let mut f = File::open(&args[1]).unwrap();
    let worker_type:&str = &args[2];
    println!("worktype: {} ", worker_type);
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    let docs = yaml::YamlLoader::load_from_str(&s).unwrap();
    let doc = &docs[0];
    // println!("{:?}", docs);
    // let sleep_time = docs["worker"]["thread"]["sleep"].as_u32().unwrap();
    let worker_sleep = doc["worker"][worker_type]["sleep"].as_i64().unwrap() as u64;
    let codes_filepath = doc["worker"][worker_type]["codes"].as_str().unwrap();
    // let idx_filepath = doc["files"]["idx"].as_str().unwrap();
    // worker.as
    // println!("test {:#?}", worker);
    // for doc in &docs {
    //     println!("---");
    //     dump_node(doc, 0);
    // }

    // let mut dataclient = QAMongoClient::new("mongodb://localhost:27017", "quantaxis");
    // let stock_day = dataclient.get_stock_day("000001", "2020-01-01", "2020-02-01");
    // println!("{:#?}", stock_day);

    let cpus = num_cpus::get();
    println!("cpus {}", cpus);
    let vecStr = get_code_split(codes_filepath).unwrap();
    let idx = vecStr.codes.len();
    let mut thread_handles = vec![];
    for str in vecStr.codes {
        thread_handles.push(
            thread::spawn( move || loop_load_data(str, worker_sleep))
        );
    }

    // Join: Wait for all threads to finish.
    for handle in thread_handles {
        handle.join().unwrap();
    }
    // thread::sleep(Duration::from_secs(3));
}

fn get_code_split<P: AsRef<Path>>(file_path: P) -> Result<SplitCode, Box<Error>> {
    // let mut vecStr:Vec<&str> = Vec::new();
    // let codeJson = read_user_from_file(filePath).unwrap();
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let codeJson:Code = serde_json::from_reader(reader)?;

    let size = codeJson.code.len();
    let mtop = f32::ceil( size as f32 / 800.0) as usize;
    let maxrc = f32::ceil( size as f32 / mtop as f32) as usize;
    // let maxrc = 800;
    // println!("mtop")
    let mut s = SplitCode{ codes: vec![] };
    for idx in 0..mtop {
        let	mut	s2	=	String::new();
        let vi = if idx == mtop - 1 {size} else {(idx+1) * maxrc};
        println!("allsize = {}, index: {} -> {}", size, idx * maxrc, vi );
        for i in &codeJson.code[idx * maxrc..vi] {
            if i.starts_with("6") {
                let s1 = format!("sh{},", i);
                s2.push_str(s1.as_str());
            } else if i.starts_with("0") || i.starts_with("3") {
                // let s1 = format!("sh{},", i).as_str();
                s2.push_str(format!("sz{},", i).as_str());
            // } else if i.starts_with("3") {
            //     s2.push_str(format!("sz{},", i).as_str());
            } else {
                s2.push_str(format!("sz{},", i).as_str());
            }
        }
        // println!("size={}", s2);
        s.codes.push(s2[..s2.len() - 1].to_string());
    }
    Ok(s)
}

fn loop_load_data(codeList:String, sleep: u64) {
    // let url = format!{"http://hq.sinajs.cn/rn=&list={}", codeList};
    let mut connection = Connection::insecure_open("amqp://admin:admin@qaeventmq:5672/").unwrap();
    let channel = connection.open_channel(None).unwrap();
    let exchange = channel
        .exchange_declare(
            ExchangeType::Direct,
            format!("stock_rs").as_str(),
            ExchangeDeclareOptions::default(),
        )
        .unwrap();

    loop {
        let local: DateTime<Local> = Local::now();
        println!("get now: {}, sleep={}", local, sleep);
        let test:Vec<&str> = codeList.split(",").collect();
        println!("code ={}", test.get(0).unwrap());
        let data = load_data(codeList.as_str()).unwrap();

        let json = serde_json::to_string(&data).unwrap();
        // println!("code ={}", json);
        exchange
            .publish(Publish::new(json.as_bytes(), ""))
            .unwrap();

        // load_data(url.to_string());
        thread::sleep(Duration::from_secs(sleep));
    }

    connection.close();
}

#[tokio::main]
async fn load_data(codeList:&str) -> Result<Vec<Realdata>, reqwest::Error> {
    let sy_time = SystemTime::now();
    let rn = sy_time.duration_since(UNIX_EPOCH).unwrap().as_millis();
    let url = format!{"http://hq.sinajs.cn/rn={}&list={}", rn, codeList};
    // let res = reqwest::get(url).await?;
    let res = reqwest::get(url).await?;
    // let res = reqwest::get("https://www.rust-lang.org/").await?;

    println!("Status: {}", res.status());

    let body = res.text().await?;
    // println!("Body:\n\n{}", body);
    //parse body
    let ret = parse_data(body);
    let ret2 = ret.unwrap();
    //send to rabbitmq

    let sy_time2 = SystemTime::now();
    println!("{}", sy_time.duration_since(UNIX_EPOCH).unwrap().as_millis());
    println!("{}", sy_time.elapsed().unwrap().as_millis());
    // println!("{}", sy_time.elapsed().unwrap().as_micros());

    // println!("{:?},{:?}", SystemTime::now().duration_since(sy_time).unwrap().as_secs(),value);
    // println!("{:?},{:?}", sy_time.elapsed().unwrap().as_secs(),value);

    Ok(ret2)
}

fn parse_data(body:String) -> Result<Vec<Realdata>, Box<Error>> {
    // let body = res.text().await?;
    // println!("Body:\n\n{}", body);
    let aa = "12345";

    let del_null_data = Regex::new(
        r#"(\w{2}\d+)=\"\";"#).unwrap();
    let head = r"(\d+)=[^\s]([^\s,]+?)";
    let mid = r",([\.\d]+)".repeat(29);
    let tail = r",([-\.\d:]+)".repeat(2);
    // let fmt = format!("{}{}", r#"(\d+)=[^\s]([^\s,]+?)"#, r#",([\.\d]+)"#.repeat(29));
    let grep_detail = Regex::new(
        format!("{}{}{}", head, mid, tail).as_str()
    ).unwrap();
    // let grep_detail = Regex::new(
    //     r"(\w{2}\d+)=[^\s]([^\s,]+?)%s%s"
    //         % (r",([\.\d]+)" * 29, r",([-\.\d:]+)" * 2)
    // ).unwrap();
    let mut ret = vec![];
    for result in grep_detail.captures_iter(body.as_bytes()) {
        let caps = result.unwrap();
        let stdata = Realdata {
            code : String::from( str::from_utf8(&caps[1]).unwrap()),
            name : String::from(str::from_utf8(&caps[2]).unwrap()),
            open: String::from(str::from_utf8(&caps[3]).unwrap()).parse::<f64>().unwrap(),
            close: String::from(str::from_utf8(&caps[4]).unwrap()).parse::<f64>().unwrap(),
            now: String::from(str::from_utf8(&caps[5]).unwrap()).parse::<f64>().unwrap(),
            price: String::from(str::from_utf8(&caps[5]).unwrap()).parse::<f64>().unwrap(),
            high: String::from(str::from_utf8(&caps[6]).unwrap()).parse::<f64>().unwrap(),
            low: String::from(str::from_utf8(&caps[7]).unwrap()).parse::<f64>().unwrap(),
            buy: String::from(str::from_utf8(&caps[8]).unwrap()).parse::<f64>().unwrap(),
            sell: String::from(str::from_utf8(&caps[9]).unwrap()).parse::<f64>().unwrap(),
            turnover: String::from(str::from_utf8(&caps[10]).unwrap()).parse::<i64>().unwrap(),
            amount: String::from(str::from_utf8(&caps[10]).unwrap()).parse::<i64>().unwrap(),
            volume: String::from(str::from_utf8(&caps[11]).unwrap()).parse::<f64>().unwrap(),
            bid1_volume: String::from(str::from_utf8(&caps[12]).unwrap()).parse::<i64>().unwrap(),
            bid1: String::from(str::from_utf8(&caps[13]).unwrap()).parse::<f64>().unwrap(),
            bid2_volume: String::from(str::from_utf8(&caps[14]).unwrap()).parse::<i64>().unwrap(),
            bid2: String::from(str::from_utf8(&caps[15]).unwrap()).parse::<f64>().unwrap(),
            bid3_volume: String::from(str::from_utf8(&caps[16]).unwrap()).parse::<i64>().unwrap(),
            bid3: String::from(str::from_utf8(&caps[17]).unwrap()).parse::<f64>().unwrap(),
            bid4_volume: String::from(str::from_utf8(&caps[18]).unwrap()).parse::<i64>().unwrap(),
            bid4: String::from(str::from_utf8(&caps[19]).unwrap()).parse::<f64>().unwrap(),
            bid5_volume: String::from(str::from_utf8(&caps[20]).unwrap()).parse::<i64>().unwrap(),
            bid5: String::from(str::from_utf8(&caps[21]).unwrap()).parse::<f64>().unwrap(),
            ask1_volume: String::from(str::from_utf8(&caps[22]).unwrap()).parse::<i64>().unwrap(),
            ask1: String::from(str::from_utf8(&caps[23]).unwrap()).parse::<f64>().unwrap(),
            ask2_volume: String::from(str::from_utf8(&caps[24]).unwrap()).parse::<i64>().unwrap(),
            ask2: String::from(str::from_utf8(&caps[25]).unwrap()).parse::<f64>().unwrap(),
            ask3_volume: String::from(str::from_utf8(&caps[26]).unwrap()).parse::<i64>().unwrap(),
            ask3: String::from(str::from_utf8(&caps[27]).unwrap()).parse::<f64>().unwrap(),
            ask4_volume: String::from(str::from_utf8(&caps[28]).unwrap()).parse::<i64>().unwrap(),
            ask4: String::from(str::from_utf8(&caps[29]).unwrap()).parse::<f64>().unwrap(),
            ask5_volume: String::from(str::from_utf8(&caps[30]).unwrap()).parse::<i64>().unwrap(),
            ask5: String::from(str::from_utf8(&caps[31]).unwrap()).parse::<f64>().unwrap(),
            date: String::from(str::from_utf8(&caps[32]).unwrap()),
            time: String::from(str::from_utf8(&caps[33]).unwrap()),
            datetime: format!("{} {}", str::from_utf8(&caps[32]).unwrap(), str::from_utf8(&caps[33]).unwrap()),
        };

        ret.push(stdata);
        // let code = str::from_utf8(&caps[1]).unwrap();
        // let name = str::from_utf8(&caps[2]).unwrap();
    }
    //parse body
    Ok(ret)
}

// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     // println!("Hello, world!");
// let resp = reqwest::get("http://hq.sinajs.cn/rn=&list=000859")
//         .await?
//         .json::<HashMap<String, String>>()
//         .await?;
//     println!("{:#?}", resp);
//     Ok(())
// }

// fn main2() -> Result<(), Box<dyn std::error::Error>> {
//     let resp = reqwest::blocking::get("https://httpbin.org/ip")?
//         .json::<HashMap<String, String>>()?;
//     println!("{:#?}", resp);
//     Ok(())
// }

// fn main2() -> Result<(), Box<dyn std::error::Error>> {
//     env_logger::init();
//
//     println!("GET https://www.rust-lang.org");
//
//     let mut res = reqwest::blocking::get("https://www.rust-lang.org/")?;
//
//     println!("Status: {}", res.status());
//     println!("Headers:\n{:?}", res.headers());
//
//     //
//     // let body = res.text()?;
//     // println!("Body:\n\n{}", body);
//
//     // copy the response body directly to stdout
//     res.copy_to(&mut std::io::stdout())?;
//
//     println!("\n\nDone.");
//     Ok(())
// }
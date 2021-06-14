use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::time::{Duration};
use imap;
use native_tls;
use stopwatch::{Stopwatch};
use std::env;
use tokio::runtime::Runtime;
use chrono;

struct EmailAccount {
    username: String,
    password: String,
}


fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3{
        println!("Usage: rust_clip [csv file] [imapserver:port]\ne.g. rust_clip accounts.csv outlook.office365.com:993");
        return;
    }
    let input_file = &args[1];
    let server_string = &args[2].split_once(":").unwrap().clone();
    let domain:&str = server_string.0;
    let port : u16 = server_string.1.parse().unwrap();


    let file_in = std::fs::File::open(input_file).expect("cannot open file");
    let file_read = BufReader::new(file_in);
    println!("File successfully read..");

    let mut data: Vec<EmailAccount> = Vec::new();
    let rt = Runtime::new().unwrap();

    for line in file_read.lines() {
        let line = line.unwrap();
        let mut ss = line.trim().split(',');
        let uname = ss.next().unwrap();
        let pwd = ss.next().unwrap();
        let e = EmailAccount { username: uname.to_string(), password: pwd.to_string() };
        if uname != "Email" {
            data.push(e);
        }
    }

    println!("{} accounts to check", data.len());
    let sw = Stopwatch::start_new();
    println!("Starting stopwatch..");
    let good = Arc::new(Mutex::new(vec![]));
    let bad = Arc::new(Mutex::new(vec![]));


    while data.len() > 0 {
        let email_account = data.pop().unwrap();
        let good_clone = Arc::clone(&good);
        let bad_clone = Arc::clone(&bad);
        let owned_domain = domain.to_string(); // or String::from_str("hello world")


        rt.spawn_blocking({
            move || {
                if check_login(&email_account, owned_domain, port) {
                    let mut v = good_clone.lock().unwrap();
                    v.push(email_account.username);
                } else {
                    let mut v = bad_clone.lock().unwrap();
                    v.push(email_account.username);
                }
            }
        });
    }

    rt.shutdown_timeout(Duration::from_secs(300));


    println!("Time taken is {}ms", sw.elapsed_ms());
    let good_accounts = good.lock().unwrap().clone();
    let bad_accounts = bad.lock().unwrap().clone();
    let good_found = good_accounts.len();
    let bad_found = bad_accounts.len();

    let now = chrono::Utc::now().format("%Y%m%d%T").to_string().replace(":", "");

    let good_file = "good_accounts_".to_owned() + &now + ".csv";
    let bad_file = "bad_accounts_".to_owned() + &now + ".csv";

    if save_vec_to_file(&good_file, good_accounts) == false{
        println!("error saving file..");
    }

    if save_vec_to_file(&bad_file, bad_accounts) == false{
        println!("error saving file..");
    }

    println!("no. of good found {} -> saved as: {}", good_found, good_file);
    println!("no. of bad found {} -> saved as: {}", bad_found, bad_file);
}


fn save_vec_to_file(file_name: &str, vec: Vec<String>) -> bool {
    let lines = vec.join("\n");
    let res = std::fs::write(file_name, &lines);
    return match res {
        Ok(_) => { true }
        Err(_) => { false }
    };
}

fn check_login(email_account: &EmailAccount, domain: String, port: u16) -> bool {
    println!("Checking {}", email_account.username);
    let tls = native_tls::TlsConnector::builder().build().unwrap();
    let client = imap::connect((domain.as_str(), port), domain.as_str(), &tls).unwrap();
    let imap_session = client.login(&email_account.username, &email_account.password);

    let mut valid: bool = true;

    match imap_session {
        Err(_error) => valid = false,
        _ => {}
    }
    valid
}
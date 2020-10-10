extern crate cymrust;
extern crate clap;

use std::io::{self, Error, ErrorKind, BufRead};
use std::process;
use cymrust::{AsNumber};
use clap::Clap;

/// asroute parses traceroute or lft outut to show summary of AS's traversed
#[derive(Clap)]
#[clap(version = "0.1", author = "Steven Pack <steven.pack.code@gmail.com>")]
struct Opts {
    // /// Some input. Because this isn't an Option<T> it's required to be used
    // input: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long)]
    verbose: bool,
}

fn main() {
  let opts: Opts = Opts::parse();
  let mut last_asn_str = String::new();

  //Read each line from stdin
  let stdin = io::stdin();
  for line in stdin.lock().lines() {        
    let line = read_line(line);   

    //For unknown * * * lines, or AS0 continue.
    if let Some(msg) = check_no_response(&line).or_else(|| check_reserved(&line)) {
      println!("{}", msg);
      continue;
    }

    let asn_str = match get_asn_str(&line) {
      Some(val) => val,
      None => {
        if opts.verbose {
          eprintln!("Couldn't find [ASN] in line. Check you passed the -a argument to traceroute. Expected usage 'traceroute -a example.com | asroute'");
        }
        continue;
      }
    };
   
    //Only lookup and show ASN when it changes
    if asn_str == last_asn_str {
      continue;
    } 

    last_asn_str = asn_str;    
    match parse_asn(&last_asn_str) {
      Ok(as_name) => println!("-> {}", as_name),
      Err(e) => {
        if opts.verbose {
          eprintln!("{}", e)
        }        
      }
    };      
  }
}

fn read_line(line: Result<String, Error>) -> String {
  match line {
    Ok(line) => line.to_uppercase(),
    Err(e) => {
      eprintln!("Failed to read line. {}", e);
      process::exit(1);
    } 
  }
}

fn check_no_response(line: &str) -> Option<&str> {
  if line.contains("*") {
    return Some("-> *")
  }
  None
}

fn check_reserved(line: &str) -> Option<&str> {
  if line.contains("[AS0]") || line.contains("[AS?]") {    
    return Some("-> AS0 (Reserved)")
  } 
  None  
}

fn parse_asn(asn_str: &str) -> Result<String, Error> {
  //Convert to anumber
  let num_str = asn_str.replace("AS", "");
  let asn: AsNumber = num_str
    .parse::<u32>()
    .map_err(|e| Error::new(ErrorKind::Other, format!("Failed to parse ASN. {}", e)))?;
    
  //Lookup via WHOIS
  let asn_info = match cymrust::cymru_asn(asn) {
    Ok(val) => val,
    Err(e) => return Err(Error::new(ErrorKind::Other, format!("Failed to lookup ASN {}, {}", asn, e)))
  };
  let as_name = if asn_info.len() > 0 {
    &asn_info[0].as_name
  } else {
    "?"
  };
  Ok(as_name.to_string())
}

fn get_asn_str(line: &str) -> Option<String> {
   //Look for [ASXXX]. Error if it's not there
   let start_index  = line.find("[").unwrap_or(usize::MAX);
   let end_index = line.find("]").unwrap_or(usize::MAX);

   if (start_index == usize::MAX) || (end_index == usize::MAX) {
     return None;
   }
   //Take the inside of the [ASXXXX]
   Some(line[start_index + 1..end_index].to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn read_line_is_upper_case() {
      let test_line = "1  [AS0] 192.168.8.1 (192.168.8.1)  3.722 ms  4.705 ms  4.613 ms";
      let expect_line = "1  [AS0] 192.168.8.1 (192.168.8.1)  3.722 MS  4.705 MS  4.613 MS";

      let line = read_line(Ok(test_line.to_string()));
      assert_eq!(expect_line, line);
  }

  #[test]
  fn check_no_response_msg() {
    let test_line_no_response = " 8  * * *";
    if let Some(msg) = check_no_response(&test_line_no_response) {
      assert_eq!(msg, "-> *")
    } else {
      assert!(false, "Expected the no response message");
    }      
  }

  #[test]
  fn check_reserved_msg() {
    let test_line = "1  [AS0] 192.168.8.1 (192.168.8.1)  3.722 ms  4.705 ms  4.613 ms";
    if let Some(msg) = check_reserved(&test_line) {
      assert_eq!(msg, "-> AS0 (Reserved)")
    } else {
      assert!(false, "Expected the reserved message");
    }      
  }

  #[test]
  fn get_asn_str() {
    let test_line = "12  [AS13335] 172.67.6.216 (172.67.6.216)  17.510 ms  16.734 ms  15.266 ms";
    let asn_str = super::get_asn_str(&test_line);
    assert!(asn_str.is_some(), "Expected a result");
    assert_eq!(asn_str.unwrap(), "AS13335", "Expected the right ASN");
  }

  #[test]
  fn parse_asn_fail() {
    let result = parse_asn(&"ASXXX");
    assert!(result.is_err(), "Didn't expect to parse ASXXXX");      
  }

  #[test]
  fn parse_asn_not_found() {
    let result = parse_asn(&"AS111111");
    println!("{:?}", result);
    assert!(result.is_err(), "Didn't expect to parse AS9999");      
  }

  #[test]
  fn parse_asn_ok() {
    let result = parse_asn(&"AS13335");
    println!("{:?}", result);
    assert!(result.is_ok(), "Cloudflare ASN lookup should succeed");
    assert!(result.unwrap().to_uppercase().contains("CLOUDFLARE"), "Expected CLOUDFLARE in the name");
  }

}
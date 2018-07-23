extern crate reqwest;
extern crate regex;

use std::io::{self, prelude::*};
use std::collections::HashMap;
use std::{env, process, f32};
use regex::Regex;

const KNOWN_BOTS: [&'static str ; 13] = [
    "Ginpachi-Sensei",
    "CR-RALEIGH|NEW",
    "CR-HOLLAND|NEW",
    "CR-BATCH|720p",
    "CR-BATCH|480p",
    "CR-BATCH|1080p",
    "CR-ARCHIVE|SD",
    "CR-ARCHIVE|720p",
    "CR-ARCHIVE|1080p",
    "Arutha|DragonBall",
    "ARUTHA-BATCH|720p",
    "ARUTHA-BATCH|480p",
    "ARUTHA-BATCH|1080p",
];


fn main() {
    //eprintln!("{:#?}", Regex::new(r"arutha.+").unwrap().captures(&String::from("AruThA-BATcH|720p").to_lowercase()).unwrap());
    //test_stuff();
    let mut info: HashMap<String, Vec<i32>> = HashMap::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        match line {
            Ok(text) => store_packinfo(text, &mut info),
            Err(e) => {
                eprintln!("Error: {}",e);
                process::exit(1);
            },
        }
    }
    if let Ok(val) = env::var("NEAREST") { // location check on? (activate by NEAREST=42)
        if val == String::from("42") {
            let locations: HashMap<String, Location> = get_bot_locations(); // <- cos we can't use literals for HMs
            let user_loc = match Location::user() {
                Ok(loc) => loc,
                Err(_) => {
                    eprintln!("Could not determine user location. Do you have internet access?");
                    process::exit(1);
                }
            };
            let mut distances: HashMap<String, f32> = HashMap::new();
            for bot in KNOWN_BOTS.into_iter() {
                let mut has_match = false;
                for rx in locations.keys() {
                    if let Some(_) = Regex::new(&rx).unwrap().captures(bot) {
                        let this_distance = user_loc.distance_to(locations.get(rx).unwrap());
                        eprintln!("{} =~ /{}/ | DISTANCE={}", bot, rx, this_distance);
                        distances.insert(String::from(*bot), this_distance);
                        has_match = true;
                        break;
                    }
                }
                if !has_match {
                    distances.insert(String::from(*bot), f32::NAN);
                }
            }
            let sorted = selection_sort_hmap(distances);
            summarize_sorted(&info, &sorted);
        }
    } else {
        summarize(&info);
    }
}

struct Location {
    lat: f32,
    long: f32,
}

impl Location {

    fn new(lat: f32, long: f32) -> Location {
        Location { lat , long }
    }

    fn distance_to(&self, l2: &Location) -> f32 {
        let sum = (l2.lat - self.lat).powi(2) + (l2.long - self.long).powi(2);
        sum.sqrt().abs()
    }

    fn user() -> Result<Location, reqwest::Error> {
        let response = reqwest::get("https://geoiptool.com/en/")?.text()?;
        let rx = Regex::new(r"\{lat: (?P<lat>-?\d+\.\d{4}), lng: (?P<lng>-?\d+\.\d{4})\}").unwrap();
        let caps = rx.captures(&response).unwrap();
        let lat: f32 = caps["lat"].parse().unwrap();
        let long:f32 = caps["lng"].parse().unwrap();

        Ok(Location::new(lat, long))
    }

}

fn selection_sort_hmap(mut hm: HashMap<String, f32>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    while hm.len() > 0 {
        let mut min = 0.0_f32;
        let mut min_k = String::new();
        for (bot, dist) in hm.iter() {
            if min_k == String::new() {
                min = *dist;
                min_k = bot.to_string();
            }
            if !dist.is_nan() {
                if dist < &min || min.is_nan() {
                    min = *dist;
                    min_k = bot.to_string();
                }
            }
        }
        let (bot,_) = hm.remove_entry(&min_k).unwrap();
        out.push(bot);
    }
    out
}

#[allow(dead_code)] // cos it's only for debug purposes
fn test_stuff() {
    match Location::user() {
        Ok(loc) => eprintln!("Found user @ ( {} | {} )", loc.lat, loc.long),
        Err(e) => eprintln!("An error occurred in Location::user(): {}", e),
    }
    process::exit(1);
}

fn get_bot_locations() -> HashMap<String, Location> {
    let mut locs: HashMap<String, Location> = HashMap::new();
    locs.insert(String::from(r"CR-ARCHIVE\|(1080p|720p|SD)"),Location::new(30.3106, -97.7227));
    locs.insert(String::from(r"Ginpachi-Sensei"),Location::new(48.8582, 2.3387));
    locs.insert(String::from(r"CR-HOLLAND\|NEW"), Location::new(52.3824, 4.8995));
    locs.insert(String::from(r"Arutha.+"), Location::new(52.3824, 4.8995));
    locs
}


fn store_packinfo(line: String, hm: &mut HashMap<String, Vec<i32>>) {
    let mut lineargs: Vec<String> = Vec::new();
    for arg in line.split_whitespace() {
        lineargs.push(String::from(arg));
    }
    let bot = String::from(&lineargs[0][0..lineargs[0].len()]);
    let packnum: i32 = lineargs[1].parse().unwrap_or_else(|_|{
        eprintln!("Failed to parse packnum: '{}' is not a number; please check formatting of input", &lineargs[1]);
        process::exit(1);
    });
    //eprintln!("{} :: {}", bot, packnum);
    if hm.contains_key(&bot) {
        let packlist = hm.get_mut(&bot).expect(&format!("Error accessing Vector for {}", bot));
        packlist.push(packnum);
        packlist.sort();
    } else {
        let mut tempvec: Vec<i32> = Vec::new();
        tempvec.push(packnum);
        hm.insert(String::from(&bot[0..bot.len()]), tempvec);
    }
}

fn bot_unknown(bot: &str) -> bool {
    for known_bot in KNOWN_BOTS.into_iter() {
        if known_bot.eq_ignore_ascii_case(bot) {
            return false;
        }
    }
    true
}

fn summarize(hm: &HashMap<String, Vec<i32>>) {
    for (bot, packs) in hm {
        let mut command = String::new();
        if packs.len() > 1 {
            command.push_str(&format!("/MSG {} XDCC BATCH ", bot));
            // grouping
            let mut i = 0;
            // testarr: [1, 2, 3, 4, 5]
            while i < packs.len() {
                let thisval = *packs.get(i).unwrap();
                if i > 0 {
                    if *packs.get(i-1).unwrap() == thisval - 1 {
                        // grouping is possible, will be attended here
                        while i+1<packs.len() && *packs.get(i+1).unwrap()==*packs.get(i).unwrap()+1 {
                            i += 1;
                        }
                        command.push_str(&format!("-{}",*packs.get(i).unwrap()));
                    } else {
                        command.push_str(&format!(",{}",thisval));
                    }
                } else {
                    command.push_str(&thisval.to_string());
                }
                i += 1;
            }
        } else {
            command.push_str(&format!("/MSG {} XDCC SEND {}", bot, packs.get(0).unwrap()));
        }
        println!("{}", command);
        if bot_unknown(bot) {
            eprintln!("NOTE: I don't seem to know '{}'. Is that bot new?\n", bot);
        }
    }
}

fn summarize_sorted(hm: &HashMap<String, Vec<i32>>, sorting: &Vec<String>) {
    for botname in sorting {
        if hm.contains_key(botname) {
            let packs = hm.get(botname).unwrap();
            let mut command = String::new();
            if packs.len() > 1 {
                command.push_str(&format!("/MSG {} XDCC BATCH ", botname));
                // grouping
                let mut i = 0;
                // testarr: [1, 2, 3, 4, 5]
                while i < packs.len() {
                    let thisval = *packs.get(i).unwrap();
                    if i > 0 {
                        if *packs.get(i-1).unwrap() == thisval - 1 {
                            // grouping is possible, will be attended here
                            while i+1<packs.len() && *packs.get(i+1).unwrap()==*packs.get(i).unwrap()+1 {
                                i += 1;
                            }
                            command.push_str(&format!("-{}",*packs.get(i).unwrap()));
                        } else {
                            command.push_str(&format!(",{}",thisval));
                        }
                    } else {
                        command.push_str(&thisval.to_string());
                    }
                    i += 1;
                }
            } else {
                command.push_str(&format!("/MSG {} XDCC SEND {}", botname, packs.get(0).unwrap()));
            }
            println!("{}", command);
            if bot_unknown(botname) {
                eprintln!("NOTE: I don't seem to know '{}'. Is that bot new?\n", botname);
            }
        }
    }
}

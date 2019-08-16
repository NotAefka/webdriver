use webdriver::session::*;
use webdriver::enums::*;
use webdriver::windows::*;
use std::rc::Rc;

static BROWSER: Browser = Browser::Firefox;

#[test]
fn navigation() {
    let webdriver = Session::new(BROWSER).expect("Echec de création de la session");
    let mut tab = webdriver.get_selected_tab().unwrap();
    tab.navigate("http://example.com/");
    assert_eq!(webdriver.get_url().unwrap(), String::from("http://example.com/"));
    tab.navigate("https://www.google.com/");
    assert_eq!(webdriver.get_url().unwrap(), String::from("https://www.google.com/"));
    webdriver.back().unwrap();
    assert_eq!(webdriver.get_url().unwrap(), String::from("http://example.com/"));
    webdriver.forward().unwrap();
    assert_eq!(webdriver.get_url().unwrap(), String::from("https://www.google.com/"));
    webdriver.refresh().unwrap();
    assert_eq!(webdriver.get_url().unwrap(), String::from("https://www.google.com/"));
}

#[test]
fn getters() {
    let webdriver = Session::new(BROWSER).expect("Echec de création de la session");
    let mut tab = webdriver.get_selected_tab().unwrap();
    tab.navigate("http://example.com/");
    assert_eq!(webdriver.get_url().unwrap(), String::from("http://example.com/"));
    assert_eq!(webdriver.get_title().unwrap(), String::from("Example Domain"));
}

#[test]
fn windows() {
    let webdriver = Session::new(BROWSER).expect("Echec de création de la session");

    let mut window1 = webdriver.get_selected_tab().unwrap();
    window1.navigate("https://www.mozilla.org/fr/").unwrap();
    assert_eq!(webdriver.get_url().unwrap(), String::from("https://www.mozilla.org/fr/"));

    let mut window2 = Tab::new(&webdriver).unwrap();
    window2.navigate("http://example.com/").unwrap();
    assert_eq!(webdriver.get_url().unwrap(), String::from("http://example.com/"));
    window1.navigate("https://www.google.com/").unwrap();
    assert_eq!(webdriver.get_url().unwrap(), String::from("https://www.google.com/"));

    println!("test");
    window2.close().unwrap();
    window1.select().unwrap();
}
//! Sessions allow you to control tabs

use std::{
    collections::HashMap,
    process::{Command, Stdio},
    rc::Rc,
    result::Result,
    thread,
    time::Duration,
};

use log::{error, info, warn};
use serde::Serialize;
use serde_json;

use crate::{enums::*, error::*, http_requests::*, tab::*, timeouts::*};

#[derive(Serialize)]
struct SessionPostData {
    capabilities: Capabilities,
}

#[derive(Serialize)]
struct Capabilities {
    #[serde(rename = "alwaysMatch")]
    always_match: AlwaysMatch,
}

#[derive(Serialize)]
struct AlwaysMatch {
    #[serde(rename = "platformName")]
    platform_name: &'static str,

    #[serde(rename = "browserName")]
    browser_name: &'static str,

    /// "moz:firefoxOptions"
    /// "goog:chromeOptions"
    #[serde(flatten)]
    browser_args: HashMap<&'static str, HeadlessArgs>,
}

#[derive(Serialize)]
struct HeadlessArgs {
    args: Vec<&'static str>,
}

/// This is the more important object.
/// Tabs can be accessed within the session.
///
/// # Example
///
/// ```rust
/// use lw_webdriver::{session::Session, enums::Browser};
///
/// let mut session = Session::new(Browser::Firefox, false).unwrap();
///
/// // accessing default tab
/// session.tabs[0].navigate("http://example.com/").unwrap();
///
/// // creating a new tab and access it
/// session.open_tab().unwrap();
/// session.tabs[1].navigate("https://mubelotix.dev/").unwrap();
/// ```
pub struct Session {
    id: Rc<String>,
    /// Contains every manually created tabs and default tab.
    /// Do not contains tabs created by web pages with javascript unless you call [update_tabs()](https://to.do/).
    pub tabs: Vec<Tab>,
    webdriver_process: Option<std::process::Child>,
}

impl Session {
    /// Create a session of a specific [browser](https://to.do/).
    /// Headless mean that the browser will be opened but not displayed (useful for servers).
    /// The crate will request a webdriver server at http://localhost:4444.
    /// If no webdriver is listening, one will be launched, but the program ([geckodriver](https://to.do/) or [chromedriver](https://to.do/))
    /// must be located at the same place than the running program.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lw_webdriver::{session::Session, enums::Browser};
    /// let mut session = Session::new(Browser::Firefox, false).unwrap();
    /// ```
    pub fn new(browser: Browser, headless: bool) -> Result<Self, WebdriverError> {
        info! {"Creating a session..."};
        let result = Session::new_session(browser, headless);

        if let Err(WebdriverError::FailedRequest) = result {
            warn!("No webdriver launched.");
            if cfg!(unix) {
                let command = match browser {
                    Browser::Firefox => "geckodriver",
                    Browser::Chrome => "chromedriver",
                };

                info!("Launching {}...", command);

                let p = Command::new(command)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .expect("Failed to start process.");

                thread::sleep(Duration::from_millis(2000));

                if let Ok(mut result) = Session::new_session(browser, headless) {
                    info!("Session created successfully.");
                    result.webdriver_process = Some(p);
                    return Ok(result);
                } else if let Err(e) = result {
                    error!("Failed to create session. error : {:?}.", e);
                    return Err(e);
                }
            } else {
                panic!("Please launch the webdriver manually.")
            }
        } else {
            return result;
        }

        result
    }

    fn new_session(browser: Browser, headless: bool) -> Result<Self, WebdriverError> {
        // Detect platform
        let platform = Platform::current();
        if let Platform::Unknow = platform {
            return Err(WebdriverError::UnsupportedPlatform);
        }

        let browser_name: &str = browser.into();
        let platform_name: &str = platform.into();

        let mut browser_args = HashMap::with_capacity(1);

        if headless {
            let headless_args = HeadlessArgs {
                args: vec!["-headless"],
            };

            browser_args.insert(
                match browser {
                    Browser::Firefox => "moz:firefoxOptions",
                    Browser::Chrome => "goog:chromeOptions",
                },
                headless_args,
            );
        }

        let post_data = SessionPostData {
            capabilities: Capabilities {
                always_match: AlwaysMatch {
                    platform_name,
                    browser_name,
                    browser_args,
                },
            },
        };

        // Send request
        let session_id = new_session(&serde_json::to_string(&post_data).unwrap())?;
        let mut session = Session {
            id: Rc::new(session_id),
            tabs: Vec::new(),
            webdriver_process: None,
        };

        session.update_tabs()?;

        Ok(session)
    }

    /// Create a new tab in the session.
    /// The tab will be directly accessible from the session (no call to [update_tabs()](https://to.do/) needed).
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lw_webdriver::{session::Session, enums::Browser};
    /// let mut session = Session::new(Browser::Firefox, false).unwrap();
    ///
    /// assert_eq!(session.tabs.len(), 1); // default tab is already opened
    /// session.open_tab().unwrap();
    /// assert_eq!(session.tabs.len(), 2); // new tab is accessible
    /// ```
    pub fn open_tab(&mut self) -> Result<usize, WebdriverError> {
        let tab_id = new_tab(&self.id)?;
        let new_tab = Tab::new_from(tab_id, Rc::clone(&self.id));
        self.tabs.push(new_tab);

        Ok(self.tabs.len() - 1)
    }

    /// When a tab is created with [open_tab()](https://to.do/) method, it is accessible directly.
    /// But sometimes a tab is created by someone else (from a web page with javascript) and you don't want to care about it!
    /// This tab will not be accessible by your program because you never asked it.
    /// However if you want to access every open tab, call this function.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use lw_webdriver::{session::Session, enums::Browser};
    /// # use std::thread::sleep;
    /// # use std::time::Duration;
    /// let mut session = Session::new(Browser::Firefox, false).unwrap();
    ///
    /// // only the default tab is open
    /// assert_eq!(session.tabs.len(), 1);
    ///
    /// // load a website
    /// session.tabs[0].navigate("https://mubelotix.dev/webdriver_tests/open_tab.html").unwrap();
    ///
    /// // observe what is happening
    /// sleep(Duration::from_secs(5));
    ///
    /// // a tab has been opened by another tab but you never asked for it
    /// // you can see two tabs displayed
    /// // but this crate don't show the useless one
    /// assert_eq!(session.tabs.len(), 1);
    ///
    /// // if you want to access it, call this function
    /// session.update_tabs().unwrap();
    ///
    /// // now you can access two tabs!
    /// assert_eq!(session.tabs.len(), 2);
    /// ```
    pub fn update_tabs(&mut self) -> Result<(), WebdriverError> {
        let tabs_id = get_open_tabs(&self.id)?;
        for tab_id in tabs_id {
            if self
                .tabs
                .iter()
                .position(|element| *element.id == tab_id)
                .is_none()
            {
                self.tabs.push(Tab::new_from(tab_id, Rc::clone(&self.id)));
            }
        }

        Ok(())
    }

    /// This is a simple method getting [timeouts](https://to.do/) of the session.
    pub fn get_timeouts(&self) -> Result<Timeouts, WebdriverError> {
        Ok(get_timeouts(&self.id)?)
    }

    /// This is a simple method setting [timeouts](https://to.do/) of the session.
    pub fn set_timeouts(&mut self, timeouts: Timeouts) -> Result<(), WebdriverError> {
        Ok(set_timeouts(&self.id, timeouts)?)
    }
}

impl PartialEq for Session {
    fn eq(&self, other: &Self) -> bool {
        self.get_id() == other.get_id()
    }
}

impl WebdriverObject for Session {
    fn get_id(&self) -> &String {
        &self.id
    }
}

impl Drop for Session {
    #[allow(unused_must_use)]
    fn drop(&mut self) {
        self.tabs.clear();
        if self.webdriver_process.is_some() {
            warn!("Killing webdriver process (may fail silently)");
            self.webdriver_process.take().unwrap().kill();
        }
    }
}

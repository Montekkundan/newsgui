#[cfg(feature = "async")]
use reqwest::Method;
use serde::Deserialize;
use url::Url;

const BASE_URL: &str = "http://localhost:8080";

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum NewsApiError {
    #[error("Failed fetching articles")]
    RequestFailed(#[from] ureq::Error),
    #[error("Failed converting response to string")]
    FailedResponseToString(#[from] std::io::Error),
    #[error("Article Parsing failed")]
    ArticleParseFailed(#[from] serde_json::Error),
    #[error("Url parsing failed")]
    UrlParsing(#[from] url::ParseError),
    #[error("Request failed: {0}")]
    BadRequest(&'static str),
    #[error("Async request failed")]
    #[cfg(feature = "async")]
    AsyncRequestFailed(#[from] reqwest::Error)
}

#[derive(Deserialize, Debug)]
pub struct Article {
    title: String,
    content: String,
    source: String,
}

impl Article {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

pub enum Endpoint {
    TopHeadlines,
}

impl ToString for Endpoint {
    fn to_string(&self) -> String {
        match self {
            Self::TopHeadlines => "articles".to_string(),
        }
    }
}

pub enum Country {
    Us,
}

impl ToString for Country {
    fn to_string(&self) -> String {
        match self {
            Self::Us => "us".to_string(),
        }
    }
}

pub struct NewsAPI {
    endpoint: Endpoint,
    country: Country,
}

impl NewsAPI {
    pub fn new() -> NewsAPI {
        NewsAPI {
            endpoint: Endpoint::TopHeadlines,
            country: Country::Us,
        }
    }

    pub fn endpoint(&mut self, endpoint: Endpoint) -> &mut NewsAPI {
        self.endpoint = endpoint;
        self
    }

    pub fn country(&mut self, country: Country) -> &mut NewsAPI {
        self.country = country;
        self
    }

    fn prepare_url(&self) -> Result<String, NewsApiError> {
        let mut url = Url::parse(BASE_URL)?;
        url.path_segments_mut()
            .unwrap()
            .push(&self.endpoint.to_string());

        Ok(url.to_string())
    }

    pub fn fetch(&self) -> Result<Vec<Article>, NewsApiError> {
        let url = self.prepare_url()?;
        let req = ureq::get(&url);
        let response: Vec<Article> = req.call()?.into_json()?;
        Ok(response)
    }

    #[cfg(feature = "async")]
    pub async fn fetch_async(&self) -> Result<Vec<Article>, NewsApiError> {
        let url = self.prepare_url()?;
        let client = reqwest::Client::new();
        let request = client
            .request(Method::GET, url)
            .header("User-Agent", "clinews")
            .build()
            .map_err(|e| NewsApiError::AsyncRequestFailed(e))?;

        let response: Vec<Article> = client
            .execute(request)
            .await?
            .json()
            .await
            .map_err(|e| NewsApiError::AsyncRequestFailed(e))?;

        Ok(response)
    }
}

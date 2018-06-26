use error::{Error, ErrorObject, RequestError};
use hyper::{
    self,
    Body,
    Request,
    Uri,
    HeaderMap,
    rt::{Future, Stream},
    client::HttpConnector,
    header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE}
};
use hyper_tls::HttpsConnector;
use serde;
use serde_json as json;
use serde_qs as qs;

#[derive(Clone, Default)]
pub struct Params {
    pub stripe_account: Option<String>,
}

#[derive(Clone)]
pub struct Client {
    client: hyper::Client<HttpsConnector<HttpConnector>>,
    secret_key: String,
    params: Params,
}

impl Client {
    fn url(path: &str) -> String {
        format!("https://api.stripe.com/v1/{}", &path[1..])
    }

    pub fn new<Str: Into<String>>(secret_key: Str) -> Client {
        let connector = HttpsConnector::new(4).unwrap();
        let client = hyper::Client::builder().build(connector);
        Client {
            client: client,
            secret_key: secret_key.into(),
            params: Params::default(),
        }
    }

    /// Clones a new client with different params.
    ///
    /// This is the recommended way to send requests for many different Stripe accounts
    /// or with different Meta, Extra, and Expand params while using the same secret key.
    pub fn with(&self, params: Params) -> Client {
        let mut client = self.clone();
        client.params = params;
        client
    }

    /// Sets a value for the Stripe-Account header
    ///
    /// This is recommended if you are acting as only one Account for the lifetime of the client.
    /// Otherwise, prefer `client.with(Params{stripe_account: "acct_ABC", ..})`.
    pub fn set_stripe_account<Str: Into<String>>(&mut self, account_id: Str) {
        self.params.stripe_account = Some(account_id.into());
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let uri: Uri = Self::url(path).parse().unwrap();
        let mut request_builder = Request::get(uri);
        for (k, v) in self.headers() {
            request_builder.header(k.unwrap().as_str(), v);
        }

        self.send(request_builder.body(Body::empty()).unwrap())
    }

    pub fn post<T: serde::de::DeserializeOwned, P: serde::Serialize>(&self, path: &str, params: P) -> Result<T, Error> {
        let uri: Uri = Self::url(path).parse().unwrap();
        let body = qs::to_string(&params)?;
        let mut request_builder = Request::post(uri);
        for (k, v) in self.headers() {
            request_builder.header(k.unwrap().as_str(), v);
        }

        self.send(request_builder.body(body.into()).unwrap())
    }

    pub fn post_empty<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let uri: Uri = Self::url(path).parse().unwrap();
        let mut request_builder = Request::post(uri);
        for (k, v) in self.headers() {
            request_builder.header(k.unwrap().as_str(), v);
        }

        self.send(request_builder.body(Body::empty()).unwrap())
    }

    pub fn delete<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let uri: Uri = Self::url(path).parse().unwrap();
        let mut request_builder = Request::delete(uri);
        for (k, v) in self.headers() {
            request_builder.header(k.unwrap().as_str(), v);
        }

        self.send(request_builder.body(Body::empty()).unwrap())
    }

    fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", self.secret_key).parse().unwrap());
        headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".parse().unwrap());
        if let Some(ref account) = self.params.stripe_account {
            headers.insert("Stripe-Account", HeaderValue::from_bytes(account.as_bytes()).unwrap());
        }
        headers
    }

    fn send<T: serde::de::DeserializeOwned>(&self, request: Request<Body>) -> Result<T, Error> {
        let response = Future::wait(self.client.request(request)).unwrap();
        let status = response.status();
        let body_fut = response.into_body()
            .map_err(|_| ())
            .fold(vec![], |mut acc, chunk| {
                acc.extend_from_slice(&chunk);
                Ok(acc)
            })
            .and_then(|v| String::from_utf8(v)
                .map_err(|_| ())
            );

        let body: String = body_fut.wait().unwrap();

        match status.as_u16() {
            200...299 => {}
            _ => {
                let mut err = json::from_str(&body).unwrap_or_else(|err| {
                    let mut req = ErrorObject { error: RequestError::default() };
                    req.error.message = Some(format!("failed to deserialize error: {}", err));
                    req
                });
                err.error.http_status = status.into();
                return Err(Error::from(err.error));
            }
        }

        json::from_str(&body).map_err(|err| Error::from(err))
    }
}

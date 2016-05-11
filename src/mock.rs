extern crate yup_hyper_mock;

use ::hyper;
use self::yup_hyper_mock::SequentialConnector;

pub fn make_mock_response(content: &str) -> hyper::client::Response {
    let mut connector = SequentialConnector::default();
    connector.content.push(content.to_string());
    let client = hyper::client::Client::with_connector(connector);
    client.get("http://127.0.0.1").send().unwrap()
}

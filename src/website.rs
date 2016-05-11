use ::mime;
use ::kuchiki;
use hyper::client::Response;
use hyper::header;
use kuchiki::traits::TendrilSink;
use html5ever::driver::BytesOpts;
use html5ever::encoding::label::encoding_from_whatwg_label;

fn parse_dom(mut response: &mut Response) -> Option<kuchiki::NodeRef> {
    match response.headers.get::<header::ContentType>() {
        Some(&header::ContentType(mime::Mime(mime::TopLevel::Text, mime::SubLevel::Html, _))) => {}
        _ => return None,
    }

    let opts = BytesOpts {
        transport_layer_encoding: response.headers
            .get::<header::ContentType>()
            .and_then(|content_type| content_type.get_param(mime::Attr::Charset))
            .and_then(|charset| encoding_from_whatwg_label(charset)),
    };

    Some(kuchiki::parse_html().from_bytes(opts).read_from(&mut response).unwrap())
}

#[derive(Debug)]
pub struct Website {
    pub request_url: String,
    pub response: Response,
    pub dom: Option<kuchiki::NodeRef>,
}

impl Website {
    pub fn from_response(url: String, mut response: Response) -> Website {
        Website {
            request_url: url,
            dom: parse_dom(&mut response),
            response: response,
        }
    }
}

#[cfg(test)]
mod website_from_response {
    use super::Website;
    use hyper::header;
    use mock::make_mock_response;

    #[test]
    fn result() {
        let website = Website::from_response("http://foo.com".to_string(),
                                             make_mock_response("HTTP/1.1 200 OK\r\n\
                                                                 Server: mock\r\n\
                                                                 \r\n\
                                                                 2"));
        assert!(website.dom.is_none());
        assert_eq!(&website.request_url, "http://foo.com");
        assert_eq!(website.response.headers.get::<header::Server>(),
                   Some(&header::Server("mock".to_string())));
    }

}

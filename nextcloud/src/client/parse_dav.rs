use quick_xml::{events::Event, Reader};

use super::{
    dav::DavError,
    prop::{
        MultiStatus, MultiStatusResponse, Prop, PropContent, PropStat, PropStatStatus, PropTag,
        UnknownStatus,
    },
};

pub fn pase_propfind(body: String) -> Result<MultiStatus, DavError> {
    let mut reader = Reader::from_str(&body);
    reader.trim_text(true);

    let mut multi_status: Option<MultiStatus> = None;

    let mut response: Option<MultiStatusResponse> = None;
    let mut propstat: Option<PropStat> = None;
    let mut propstat_status: Option<PropStatStatus> = None;
    let mut prop_list: Option<Prop> = None;
    let mut prop: Option<Prop> = None;

    let mut stack: Vec<PropTag> = Vec::new();

    loop {
        match reader.read_event().unwrap() {
            Event::Start(e) => {
                let prop_tag = PropTag::from(e.name());

                if prop_tag.namespace == "d"
                    && prop_tag.name == "multistatus"
                    && multi_status.is_none()
                {
                    // d:multistatus is the root element
                    multi_status = Some(MultiStatus {
                        responses: Vec::new(),
                    });
                } else if prop_tag.namespace == "d"
                    && prop_tag.name == "response"
                    && response.is_none()
                {
                    // d:response is a child of multistatus
                    response = Some(MultiStatusResponse {
                        href: "".to_string(),
                        prop_stats: Vec::new(),
                        response_description: None,
                    });
                } else if prop_tag.namespace == "d"
                    && prop_tag.name == "propstat"
                    && propstat.is_none()
                {
                    // d:propstat is a child of response
                    propstat = Some(PropStat {
                        status: PropStatStatus::Ok,
                        props: Vec::new(),
                    });
                } else if prop_tag.namespace == "d"
                    && prop_tag.name == "status"
                    && propstat_status.is_none()
                {
                    // d:status is a child of propstat
                    propstat_status = Some(PropStatStatus::Unknown(UnknownStatus::Unknown));
                } else if prop_tag.namespace == "d" && prop_tag.name == "response-description" {
                    // d:response-description is a child of response
                    // ...but we don't care about it
                } else if prop_tag.namespace == "d"
                    && prop_tag.name == "prop"
                    && prop_list.is_none()
                {
                    // d:prop is a child of propstat
                    prop_list = Some(Prop {
                        tag: prop_tag.clone(),
                        content: PropContent::Empty,
                    });
                } else if prop_list.is_some() && prop.is_none() {
                    // Anything else is a child of propstat_prop
                    prop = Some(Prop {
                        tag: prop_tag.clone(),
                        content: PropContent::Empty,
                    });
                }

                stack.push(prop_tag);
            }
            Event::End(e) => {
                let prop_tag = PropTag::from(e.name());

                if prop_tag.namespace == "d" && prop_tag.name == "response" {
                    // If we have a response, add it to the multi_status
                    if let Some(ref mut m) = multi_status {
                        if let Some(ref r) = response {
                            m.responses.push(r.clone());

                            response = None;
                        }
                    }
                } else if prop_tag.namespace == "d" && prop_tag.name == "propstat" {
                    // If we have a propstat, add it to the response
                    if let Some(ref mut r) = response {
                        if let Some(ref mut p) = propstat {
                            r.prop_stats.push(p.clone());

                            propstat = None;
                        }
                    }
                } else if prop_tag.namespace == "d" && prop_tag.name == "status" {
                    // If we have a status, add it to the propstat
                    if let Some(ref mut p) = propstat {
                        if let Some(ref mut s) = propstat_status {
                            p.status = s.clone();

                            propstat_status = None;
                        }
                    }
                } else if prop_tag.namespace == "d" && prop_tag.name == "response-description" {
                    // ignored
                } else if prop_tag.namespace == "d" && prop_tag.name == "prop" {
                    // If we have a prop list, add it to the propstat
                    if let Some(ref mut ps) = propstat {
                        if let Some(ref mut pl) = prop_list {
                            ps.props.push(pl.clone());

                            prop_list = None;
                        }
                    }
                } else {
                    if let Some(ref mut pl) = prop_list {
                        // If we have a prop, add it to the prop list
                        if let Some(ref p) = prop {
                            match pl.content {
                                PropContent::Text(_) | PropContent::Empty => {
                                    pl.content = PropContent::Props(vec![p.clone()]);
                                }
                                PropContent::Props(ref mut props) => {
                                    props.push(p.clone());
                                }
                            }

                            prop = None;
                        }
                    }
                }

                stack.pop();
            }
            Event::Text(e) => match stack.last() {
                Some(tag) => {
                    // d:href, d:status, and props can have text content

                    if tag.namespace == "d" && tag.name == "href" {
                        if let Some(ref mut r) = response {
                            r.href = match e.unescape() {
                                Ok(h) => h.to_string(),
                                Err(_) => "".to_string(),
                            };
                        }
                    } else if tag.namespace == "d" && tag.name == "status" {
                        if let Some(ref mut r) = response {
                            // TODO: parse status
                            propstat_status = Some(PropStatStatus::Ok);
                        }
                    } else if let Some(ref mut p) = prop {
                        p.content = PropContent::Text(match e.unescape() {
                            Ok(t) => t.to_string(),
                            Err(_) => "".to_string(),
                        });
                    }
                }
                None => (),
            },
            Event::Eof => break,
            _ => (),
        }
    }

    match multi_status {
        Some(m) => Ok(m),
        None => Err(DavError::NoContent),
    }
}

#[test]
fn test_parse() {
    println!(
        "{:#?}",
        pase_propfind(include_str!("../../../text.xml").to_string())
    );
}

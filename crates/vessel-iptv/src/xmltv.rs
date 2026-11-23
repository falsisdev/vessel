//! Streaming XMLTV EPG parser (`quick-xml`) and Now/Next schedule helper.

use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};
use vessel_domain::{VesselError, VesselResult};

use crate::{EpgNowNext, EpgProgram};

/// Howard Hinnant's civil-to-days algorithm (days since 1970-01-01 Unix epoch).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * m + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Parses an XMLTV timestamp (`YYYYMMDDHHMMSS [+HHMM|-HHMM]`) into UTC Unix epoch seconds.
pub fn parse_xmltv_timestamp(raw: &str) -> Option<i64> {
    let trimmed = raw.trim();
    let mut parts = trimmed.split_whitespace();
    let dt = parts.next()?;
    if dt.len() < 12 || !dt.is_ascii() {
        return None;
    }

    let year: i64 = dt.get(0..4)?.parse().ok()?;
    let month: i64 = dt.get(4..6)?.parse().ok()?;
    let day: i64 = dt.get(6..8)?.parse().ok()?;
    let hour: i64 = dt.get(8..10)?.parse().ok()?;
    let minute: i64 = dt.get(10..12)?.parse().ok()?;
    let second: i64 = dt.get(12..14).and_then(|s| s.parse().ok()).unwrap_or(0);

    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=60).contains(&second)
    {
        return None;
    }

    let offset_secs: i64 = match parts.next() {
        Some(tz) if tz.len() == 5 && tz.is_ascii() => {
            let sign: i64 = match tz.as_bytes()[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let tz_h: i64 = tz.get(1..3)?.parse().ok()?;
            let tz_m: i64 = tz.get(3..5)?.parse().ok()?;
            sign * (tz_h * 3600 + tz_m * 60)
        }
        Some("UTC") | Some("Z") | None => 0,
        _ => return None,
    };

    let days = days_from_civil(year, month, day);
    let local_epoch = days * 86_400 + hour * 3600 + minute * 60 + second;
    Some(local_epoch - offset_secs)
}

#[derive(Debug, Default)]
struct ActiveProgramme {
    channel_tvg_id: String,
    start_ts: i64,
    end_ts: i64,
    title: String,
    description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveField {
    Title,
    Desc,
}

/// Streams an XMLTV document into [`EpgProgram`] records using `quick-xml`.
pub fn parse_xmltv_programs(xml: &str) -> VesselResult<Vec<EpgProgram>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);

    let mut programs = Vec::new();
    let mut active: Option<ActiveProgramme> = None;
    let mut active_field: Option<ActiveField> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.local_name().as_ref() {
                "programme" => {
                    let mut channel = None;
                    let mut start = None;
                    let mut stop = None;
                    for attr in tag.attributes() {
                        let attr = attr.map_err(|err| {
                            VesselError::InvalidInput(format!("malformed XMLTV attribute: {err}"))
                        })?;
                        let val =
                            attr.normalized_value(XmlVersion::Explicit1_0)
                                .map_err(|err| {
                                    VesselError::InvalidInput(format!(
                                        "invalid XMLTV entity: {err}"
                                    ))
                                })?;
                        match attr.key.local_name().as_ref() {
                            "channel" => channel = Some(val.into_owned()),
                            "start" => start = parse_xmltv_timestamp(&val),
                            "stop" => stop = parse_xmltv_timestamp(&val),
                            _ => {}
                        }
                    }
                    if let (Some(ch), Some(s), Some(e)) = (channel, start, stop)
                        && !ch.trim().is_empty()
                    {
                        active = Some(ActiveProgramme {
                            channel_tvg_id: ch.trim().to_string(),
                            start_ts: s,
                            end_ts: e,
                            title: String::new(),
                            description: String::new(),
                        });
                    }
                }
                "title" if active.is_some() => active_field = Some(ActiveField::Title),
                "desc" if active.is_some() => active_field = Some(ActiveField::Desc),
                _ => {}
            },
            Ok(Event::Text(text)) => {
                if let (Some(prog), Some(field)) = (active.as_mut(), active_field) {
                    let content = text.xml10_content();
                    match field {
                        ActiveField::Title => prog.title.push_str(&content),
                        ActiveField::Desc => prog.description.push_str(&content),
                    }
                }
            }
            Ok(Event::CData(cdata)) => {
                if let (Some(prog), Some(field)) = (active.as_mut(), active_field) {
                    let content = cdata.xml10_content();
                    match field {
                        ActiveField::Title => prog.title.push_str(&content),
                        ActiveField::Desc => prog.description.push_str(&content),
                    }
                }
            }
            Ok(Event::GeneralRef(entity)) => {
                if let (Some(prog), Some(field)) = (active.as_mut(), active_field) {
                    let target = match field {
                        ActiveField::Title => &mut prog.title,
                        ActiveField::Desc => &mut prog.description,
                    };
                    if let Ok(Some(ch)) = entity.resolve_char_ref() {
                        target.push(ch);
                    } else if let Some(resolved) = resolve_predefined_entity(entity.as_ref()) {
                        target.push_str(resolved);
                    }
                }
            }
            Ok(Event::End(tag)) => match tag.local_name().as_ref() {
                "title" | "desc" => active_field = None,
                "programme" => {
                    active_field = None;
                    if let Some(prog) = active.take() {
                        let title = prog.title.trim().to_string();
                        if !title.is_empty() {
                            let desc = prog.description.trim();
                            programs.push(EpgProgram {
                                channel_tvg_id: prog.channel_tvg_id,
                                title,
                                description: (!desc.is_empty()).then(|| desc.to_string()),
                                start_ts: prog.start_ts,
                                end_ts: prog.end_ts,
                            });
                        }
                    }
                }
                _ => {}
            },
            Ok(_) => {}
            Err(err) => {
                return Err(VesselError::InvalidInput(format!(
                    "failed parsing XMLTV stream: {err}"
                )));
            }
        }
    }

    Ok(programs)
}

/// Computes the active (`now`) and upcoming (`next`) program from a channel's EPG list.
pub fn now_next_from_programs(programs: &[EpgProgram], now_ts: i64) -> EpgNowNext {
    let mut now: Option<EpgProgram> = None;
    let mut next: Option<EpgProgram> = None;

    for prog in programs {
        if prog.start_ts <= now_ts && prog.end_ts > now_ts {
            now = Some(prog.clone());
        } else if prog.start_ts > now_ts
            && next.as_ref().is_none_or(|cur| prog.start_ts < cur.start_ts)
        {
            next = Some(prog.clone());
        }
    }

    EpgNowNext { now, next }
}

#[cfg(test)]
mod tests {
    use super::{now_next_from_programs, parse_xmltv_programs, parse_xmltv_timestamp};

    #[test]
    fn parses_xmltv_timestamps_with_timezone_offsets() {
        let utc_ts = parse_xmltv_timestamp("20261123120000 +0000").expect("utc");
        let ist_ts = parse_xmltv_timestamp("20261123150000 +0300").expect("+0300");
        assert_eq!(utc_ts, ist_ts);

        let ny_ts = parse_xmltv_timestamp("20261123070000 -0500").expect("-0500");
        assert_eq!(utc_ts, ny_ts);
    }

    #[test]
    fn parses_xmltv_programmes_entities_and_computes_now_next() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <tv generator-info-name="anthology-xmltv">
          <channel id="news.one"><display-name>News One</display-name></channel>
          <programme start="20261123120000 +0000" stop="20261123130000 +0000" channel="news.one">
            <title lang="en">Midday Report &amp; Analysis</title>
            <desc>Live coverage &#38; studio panel.</desc>
          </programme>
          <programme start="20261123130000 +0000" stop="20261123140000 +0000" channel="news.one">
            <title lang="en">Afternoon Briefing</title>
          </programme>
        </tv>"#;

        let programs = parse_xmltv_programs(xml).expect("parse xmltv");
        assert_eq!(programs.len(), 2);
        assert_eq!(programs[0].channel_tvg_id, "news.one");
        assert_eq!(programs[0].title, "Midday Report & Analysis");
        assert_eq!(
            programs[0].description.as_deref(),
            Some("Live coverage & studio panel.")
        );
        assert_eq!(programs[1].description, None);

        let mid = parse_xmltv_timestamp("20261123123000 +0000").expect("mid");
        let nn = now_next_from_programs(&programs, mid);
        assert_eq!(
            nn.now.as_ref().map(|p| p.title.as_str()),
            Some("Midday Report & Analysis")
        );
        assert_eq!(
            nn.next.as_ref().map(|p| p.title.as_str()),
            Some("Afternoon Briefing")
        );
    }
}

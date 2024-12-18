use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::fs::File;
use std::path::Path;

use crate::submitter::StepResult;

use anyhow::Error;
use std::fs::create_dir_all;

pub fn create_junit(
    results: &[StepResult],
    file_path: &Path,
    hostname: Option<&str>,
) -> Result<(), Error> {
    if let Some(parent) = file_path.parent() {
        create_dir_all(parent)?;
    }

    let file = File::create(file_path)?;

    let mut writer = Writer::new_with_indent(file, b' ', 4);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

    // Add in the testsuite elem

    let test_num = results.len();
    let skip_num = results
        .iter()
        .filter(|step| {
            if let Some(ref output) = step.error {
                return output == "Dependency Not Met";
            }
            false
        })
        .count();
    let failure_num = results.iter().filter(|step| !step.pass).count() - skip_num;

    let time = results
        .iter()
        .fold(0f32, |sum, step| sum + (step.duration / 1000f32));

    let hostname = match hostname {
        Some(hostname) => String::from(hostname),
        None => hostname::get()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|_| String::from("")),
    };

    let testsuite_content = "testsuite";
    let mut testsuite = BytesStart::from_content(testsuite_content, testsuite_content.len());

    testsuite.push_attribute(("name", "lorikeet"));
    testsuite.push_attribute(("hostname", &*hostname));

    testsuite.push_attribute(("tests", &*test_num.to_string()));
    testsuite.push_attribute(("failures", &*failure_num.to_string()));
    testsuite.push_attribute(("skipped", &*skip_num.to_string()));
    testsuite.push_attribute(("time", &*time.to_string()));

    writer.write_event(Event::Start(testsuite))?;

    for result in results.iter() {
        let testcase_content = "testcase";
        let mut testcase = BytesStart::from_content(testcase_content, testcase_content.len());

        testcase.push_attribute(("name", &*result.name));

        if let Some(ref desc) = result.description {
            testcase.push_attribute(("classname", desc as &str));
        } else {
            testcase.push_attribute(("classname", ""));
        }

        testcase.push_attribute(("time", &*(result.duration / 1000f32).to_string()));

        writer.write_event(Event::Start(testcase))?;

        let system_out_content = "system-out";

        writer.write_event(Event::Start(BytesStart::from_content(
            system_out_content,
            system_out_content.len(),
        )))?;

        writer.write_event(Event::Text(BytesText::new(
            &filter_invalid_chars(&result.output),
        )))?;

        writer.write_event(Event::End(BytesEnd::new(system_out_content)))?;

        if !result.pass {
            let error_text = result.error.as_deref().unwrap_or("");

            if error_text == "Dependency Not Met" {
                let skipped_content = "skipped";
                let mut skipped = BytesStart::from_content(skipped_content, skipped_content.len());
                skipped.push_attribute(("message", "Dependency Not Met"));

                writer.write_event(Event::Start(skipped))?;

                writer.write_event(Event::End(BytesEnd::new(skipped_content)))?;
            } else {
                let failure_content = "failure";
                let mut failure = BytesStart::from_content(failure_content, failure_content.len());
                failure.push_attribute(("message", "Step failed to finish"));

                writer.write_event(Event::Start(failure))?;
                writer.write_event(Event::Text(BytesText::new(
                    &filter_invalid_chars(&error_text),
                )))?;
                writer.write_event(Event::End(BytesEnd::new(failure_content)))?;
            }
        }

        writer.write_event(Event::End(BytesEnd::new(testcase_content)))?;
    }

    writer.write_event(Event::End(BytesEnd::new(testsuite_content)))?;

    Ok(())
}

fn filter_invalid_chars(input: &str) -> String {
    let mut output = String::new();

    for ch in input.chars() {
        if ('\u{0020}'..='\u{D7FF}').contains(&ch)
            || ('\u{E000}'..='\u{FFFD}').contains(&ch)
            || ch == '\u{0009}'
            || ch == '\u{0A}'
            || ch == '\u{0D}'
        {
            output.push(ch);
        }
    }

    output
}

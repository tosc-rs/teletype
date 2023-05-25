use core::fmt::Write;
use input_mgr::{RingLine, Source};
use textwrap::dedent;

#[test]
fn basic_usage() {
    // Create a ringline buffer with 80 characters per line, and 16 lines
    let mut ringline = RingLine::<16, 80>::new();

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            ====
        "#
        )
        .trim(),
    );

    // Push some contents to the user buffer
    b"hello from local!".iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L# | hello from local!
            ====
        "#
        )
        .trim(),
    );

    // Push some contents into the remote
    b"hello from remote!".iter().for_each(|c| {
        ringline.append_remote_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            R# | hello from remote!
            L# | hello from local!
            ====
        "#
        )
        .trim(),
    );

    // Mark the local contents as submitted
    ringline.submit_local_editing();

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L. | hello from local!
            R# | hello from remote!
            ====
        "#
        )
        .trim(),
    );

    // Push some contents to the user buffer
    b"hello from local2!".iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L. | hello from local!
            R# | hello from remote!
            L# | hello from local2!
            ====
        "#
        )
        .trim(),
    );

    // Mark the remote contents as submitted
    ringline.submit_remote_editing();

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L. | hello from local!
            R. | hello from remote!
            L# | hello from local2!
            ====
        "#
        )
        .trim(),
    );

    // Push some contents into the remote
    b"hello from remote2!".iter().for_each(|c| {
        ringline.append_remote_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L. | hello from local!
            R. | hello from remote!
            R# | hello from remote2!
            L# | hello from local2!
            ====
        "#
        )
        .trim(),
    );

    // Mark the local contents as submitted
    ringline.submit_local_editing();

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L. | hello from local!
            R. | hello from remote!
            L. | hello from local2!
            R# | hello from remote2!
            ====
        "#
        )
        .trim(),
    );

    // Mark the remote contents as submitted
    ringline.submit_remote_editing();

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L. | hello from local!
            R. | hello from remote!
            L. | hello from local2!
            R. | hello from remote2!
            ====
        "#
        )
        .trim(),
    );
}

#[test]
fn multiline() {
    // Create a ringline buffer with 80 characters per line, and 16 lines
    let mut ringline = RingLine::<16, 80>::new();

    let fifteen = b"....^....^....^";

    // Push some contents to the user buffer (15)
    fifteen.iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L# | ....^....^....^
            ====
        "#
        )
        .trim(),
    );

    // Push some contents to the user buffer (30)
    fifteen.iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L# | ....^....^....^....^....^....^
            ====
        "#
        )
        .trim(),
    );

    // Push some contents to the user buffer (45)
    fifteen.iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });
    // Push some contents to the user buffer (60)
    fifteen.iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });
    // Push some contents to the user buffer (75)
    fifteen.iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L# | ....^....^....^....^....^....^....^....^....^....^....^....^....^....^....^
            ====
        "#
        )
        .trim(),
    );

    // Push some contents to the user buffer (90)
    fifteen.iter().for_each(|c| {
        ringline.append_local_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L# | ....^....^....^....^....^....^....^....^....^....^....^....^....^....^....^....^
            L# | ....^....^
            ====
        "#
        )
        .trim(),
    );

    // Push some contents into the remote
    b"hello from remote!".iter().for_each(|c| {
        ringline.append_remote_char(*c).unwrap();
    });

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            R# | hello from remote!
            L# | ....^....^....^....^....^....^....^....^....^....^....^....^....^....^....^....^
            L# | ....^....^
            ====
        "#
        )
        .trim(),
    );

    ringline.submit_local_editing();

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            L. | ....^....^....^....^....^....^....^....^....^....^....^....^....^....^....^....^
            L. | ....^....^
            R# | hello from remote!
            ====
        "#
        )
        .trim(),
    );

    ringline.submit_remote_editing();
}

#[test]
fn interleaved() {
    // Create a ringline buffer with 80 characters per line, and 16 lines
    let mut ringline = RingLine::<16, 80>::new();

    for i in 0..8 {
        // Push some contents into the remote
        format!("hello from remote {i}")
            .as_bytes()
            .iter()
            .for_each(|c| {
                ringline.append_remote_char(*c).unwrap();
            });
        ringline.submit_remote_editing();
        // Push some contents into the local
        format!("hello from local {i}")
            .as_bytes()
            .iter()
            .for_each(|c| {
                ringline.append_local_char(*c).unwrap();
            });
        ringline.submit_local_editing();
    }

    let dump = dump_to_string(&ringline);
    assert_eq!(
        dump,
        dedent(
            r#"
            ====
            R. | hello from remote 0
            L. | hello from local 0
            R. | hello from remote 1
            L. | hello from local 1
            R. | hello from remote 2
            L. | hello from local 2
            R. | hello from remote 3
            L. | hello from local 3
            R. | hello from remote 4
            L. | hello from local 4
            R. | hello from remote 5
            L. | hello from local 5
            R. | hello from remote 6
            L. | hello from local 6
            R. | hello from remote 7
            L. | hello from local 7
            ====
        "#
        )
        .trim(),
    );
}

fn dump_to_string(ringline: &RingLine<16, 80>) -> String {
    let mut out = String::new();
    writeln!(&mut out, "====").unwrap();
    // Iterate through all the "latched" messages.
    //
    // These are newest to oldest! That is annoying!
    for item in ringline
        .iter_history()
        .map(|l| (l.status, l.as_str()))
        .collect::<Vec<_>>()
        .iter()
        .rev()
    {
        match item.0 {
            Source::Local => {
                writeln!(&mut out, "L. | {}", item.1).unwrap();
            }
            Source::Remote => {
                writeln!(&mut out, "R. | {}", item.1).unwrap();
            }
        }
    }

    // Then show the current "remote" working buffer
    //
    // These are newest to oldest! That is annoying!
    for item in ringline
        .iter_remote_editing()
        .map(|l| l.as_str())
        .collect::<Vec<_>>()
        .iter()
        .rev()
    {
        writeln!(&mut out, "R# | {}", item).unwrap();
    }

    // Then show the current "local" working buffer
    //
    // These are newest to oldest! That is annoying!
    for item in ringline
        .iter_local_editing()
        .map(|l| l.as_str())
        .collect::<Vec<_>>()
        .iter()
        .rev()
    {
        writeln!(&mut out, "L# | {}", item).unwrap();
    }

    write!(&mut out, "====").unwrap();

    out
}

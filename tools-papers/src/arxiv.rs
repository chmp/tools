//! Helper to handle papers from arxiv
//!
use std::collections::HashMap;
use lazy_static::lazy_static;
use regex::Regex;

/// Return true if the passed string is the filename of an arXiv paper
pub fn is_arxiv_paper(s: &str) -> bool {
    lazy_static! {
        static ref PATTERN: Regex = Regex::new(r##"^\d{4}\.\d*(v\d+)?$"##).unwrap();
    }
    PATTERN.is_match(s)
}

/// Parse the metadata of the arxiv text format
pub fn parse_arxiv_metadata(s: &str) -> Option<HashMap<&str, &str>> {
    split_arxiv_metadata(s).map(|(header, abstract_)| {
        let mut data = parse_arxiv_header(header);
        data.insert("abstract", abstract_);
        data
    })
}

fn split_arxiv_metadata(s: &str) -> Option<(&str, &str)> {
    let start_header = s.find(r"\\")?;
    let start_header = start_header + 2;

    let start_abstract = s[start_header..].find(r"\\")?;
    let end_header = start_header + start_abstract;
    let start_abstract = start_header + start_abstract + 2;

    let end_abstract = s[start_abstract..].find(r"\\");
    let end_abstract = end_abstract
        .map(|v| v + start_abstract)
        .or_else(|| Some(s.len()))
        .unwrap();

    Some((
        &s[start_header..end_header],
        s[start_abstract..end_abstract].trim(),
    ))
}

/// Parses the header information in the arxiv text format
///
/// # Arguments
///
/// * `s` - the content of the header
///
fn parse_arxiv_header(s: &str) -> HashMap<&str, &str> {
    enum HeaderParserState {
        ParseKey(usize),
        ParseValue(usize),
        AfterNewLine(usize),
    }

    let mut result = HashMap::<&str, &str>::new();
    let mut state = HeaderParserState::ParseKey(0);
    let mut current_key: &str = &s[0..0];

    for (i, c) in s.char_indices() {
        match state {
            HeaderParserState::ParseKey(start) => {
                if c == ':' {
                    current_key = &s[start..i].trim();
                    state = HeaderParserState::ParseValue(i + 1);
                }
            }
            HeaderParserState::ParseValue(start) => {
                if c == '\n' {
                    state = HeaderParserState::AfterNewLine(start);
                }
            }
            HeaderParserState::AfterNewLine(start) => {
                if c.is_whitespace() && c != '\n' {
                    state = HeaderParserState::ParseValue(start)
                } else {
                    result.insert(current_key, &s[start..i - 1].trim());
                    state = HeaderParserState::ParseKey(i);
                }
            }
        }
    }

    match state {
        HeaderParserState::AfterNewLine(start) => {
            result.insert(current_key, &s[start..].trim());
        }
        HeaderParserState::ParseKey(start) => {
            result.insert(current_key, &s[start..].trim());
        }
        _ => {}
    }

    result
}

#[cfg(test)]
mod is_arxiv_paper_tests {
    use super::is_arxiv_paper;

    #[test]
    fn example() {
        assert_eq!(true, is_arxiv_paper("1706.03762v3"));
        assert_eq!(
            false,
            is_arxiv_paper("2ef4811bc3112c2561c8e666b15980d8ca4700e6")
        );
    }
}

#[cfg(test)]
mod parse_arxiv_metadata_tests {
    use super::parse_arxiv_metadata;

    #[test]
    fn example() {
        let metadata = r##"------------------------------------------------------------------------------
\\
arXiv:1706.03762
From: Ashish Vaswani
Date: Mon, 12 Jun 2017 17:57:34 GMT   (1102kb,D)
Date (revised v2): Mon, 19 Jun 2017 16:49:45 GMT   (1125kb,D)
Date (revised v3): Tue, 20 Jun 2017 05:20:02 GMT   (1125kb,D)

Title: Attention Is All You Need
Authors: Ashish Vaswani, Noam Shazeer, Niki Parmar, Jakob Uszkoreit, Llion
    Jones, Aidan N. Gomez, Lukasz Kaiser, Illia Polosukhin
Categories: cs.CL cs.LG
Comments: 15 pages, 5 figure
License: http://arxiv.org/licenses/nonexclusive-distrib/1.0/
\\
    The dominant sequence transduction models are based on complex recurrent or
convolutional neural networks in an encoder-decoder configuration. The best
performing models also connect the encoder and decoder through an attention
mechanism. We propose a new simple network architecture, the Transformer, based
solely on attention mechanisms, dispensing with recurrence and convolutions
entirely. Experiments on two machine translation tasks show these models to be
superior in quality while being more parallelizable and requiring significantly
less time to train. Our model achieves 28.4 BLEU on the WMT 2014
English-to-German translation task, improving over the existing best results,
including ensembles by over 2 BLEU. On the WMT 2014 English-to-French
translation task, our model establishes a new single-model state-of-the-art
BLEU score of 41.0 after training for 3.5 days on eight GPUs, a small fraction
of the training costs of the best models from the literature. We show that the
Transformer generalizes well to other tasks by applying it successfully to
English constituency parsing both with large and limited training data.
\\"##;
        let data = parse_arxiv_metadata(metadata).unwrap();
        println!("keys: {:?}", data.keys().collect::<Vec<_>>());
        assert_eq!(data["Title"], "Attention Is All You Need");
    }

    #[test]
    fn example2() {
        let metadata = r##"------------------------------------------------------------------------------
\\
arXiv:1310.1757
From: Iain Murray
Date: Mon, 7 Oct 2013 12:42:41 GMT   (357kb,D)
Date (revised v2): Sat, 11 Jan 2014 17:13:56 GMT   (360kb,D)

Title: A Deep and Tractable Density Estimator
Authors: Benigno Uria, Iain Murray, Hugo Larochelle
Categories: stat.ML cs.LG
Comments: 9 pages, 4 tables, 1 algorithm, 5 figures. To appear ICML 2014, JMLR
    W&CP volume 32
License: http://arxiv.org/licenses/nonexclusive-distrib/1.0/
\\
    The Neural Autoregressive Distribution Estimator (NADE) and its real-valued
version RNADE are competitive density models of multidimensional data across a
variety of domains. These models use a fixed, arbitrary ordering of the data
dimensions. One can easily condition on variables at the beginning of the
ordering, and marginalize out variables at the end of the ordering, however
other inference tasks require approximate inference. In this work we introduce
an efficient procedure to simultaneously train a NADE model for each possible
ordering of the variables, by sharing parameters across all these models. We
can thus use the most convenient model for each inference task at hand, and
ensembles of such models with different orderings are immediately available.
Moreover, unlike the original NADE, our training procedure scales to deep
models. Empirically, ensembles of Deep NADE models obtain state of the art
density estimation performance.
\\"##;
        let data = parse_arxiv_metadata(metadata).unwrap();
        println!("keys: {:?}", data.keys().collect::<Vec<_>>());
        assert_eq!(data["Title"], "A Deep and Tractable Density Estimator");
    }
}

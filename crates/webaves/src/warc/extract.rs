//! Document extraction.

use std::io::Read;

use crate::{
    http::{field::MediaType, MessageReader},
    io::ComboReader,
};

use super::{HeaderMapExt, HeaderMetadata};

/// Determine whether an extractor can handle a record.
pub trait Classifier {
    /// Returns true if the extractor can extract documents given the header.
    fn can_accept(&self, metadata: &HeaderMetadata) -> bool;
}

/// Extracts a document from a record.
pub trait Extractor<S: Read>: Read {
    /// Returns a reference to the wrapped stream.
    fn get_ref(&self) -> &S;
    /// Returns a mutable reference to the wrapped stream.
    fn get_mut(&mut self) -> &mut S;
    /// Returns the wrapped stream.
    fn into_inner(self) -> S;
    /// Returns the wrapped stream.
    fn into_inner_box(self: Box<Self>) -> S;
    /// Checks for any errors and returns the wrapped stream.
    fn finish(self) -> Result<S, crate::error::Error>;
    /// Checks for any errors and returns the wrapped stream.
    fn finish_box(self: Box<Self>) -> Result<S, crate::error::Error>;
}

/// Creates an extractor.
pub type ExtractorFactory<'a, S> =
    Box<dyn 'a + Fn(S) -> Result<Box<dyn 'a + Extractor<S>>, crate::error::Error>>;

/// Dispatcher for multiple extractors.
pub struct ExtractorDispatcher<'a, S: Read> {
    source: Option<S>,
    extractor: Option<Box<dyn 'a + Extractor<S>>>,
    extractors: Vec<(Box<dyn 'a + Classifier>, ExtractorFactory<'a, S>)>,
}

impl<'a, S: 'a + Read> ExtractorDispatcher<'a, S> {
    /// Create an empty `ExtractorDispatcher` with the given input stream.
    pub fn new(source: S) -> Self {
        Self {
            source: Some(source),
            extractor: None,
            extractors: Vec::new(),
        }
    }

    /// Returns a reference to the wrapped stream.
    pub fn get_ref(&self) -> &S {
        match &self.source {
            Some(source) => source,
            None => self.extractor.as_ref().unwrap().get_ref(),
        }
    }

    /// Returns a mutable reference to the wrapped stream.
    pub fn get_mut(&mut self) -> &mut S {
        match &mut self.source {
            Some(source) => source,
            None => self.extractor.as_mut().unwrap().get_mut(),
        }
    }

    /// Returns the wrapped stream.
    pub fn into_inner(self) -> S {
        match self.source {
            Some(source) => source,
            None => self.extractor.unwrap().into_inner_box(),
        }
    }

    /// Add a classifier and an extractor factory that will be searched.
    pub fn add_extractor(
        &mut self,
        classifier: Box<dyn Classifier>,
        extractor_factory: ExtractorFactory<'a, S>,
    ) {
        self.extractors.push((classifier, extractor_factory));
    }

    /// Adds default classifiers and extractors to this object.
    pub fn add_default_extractors(&mut self) {
        self.add_extractor(
            Box::new(ResourceClassifier),
            Box::new(|source: S| Ok(Box::new(ResourceExtractor::new(source)?))),
        );
        self.add_extractor(
            Box::new(HTTPClassifier),
            Box::new(|source: S| Ok(Box::new(HTTPExtractor::new(source)?))),
        );
    }

    /// Returns whether any contained extractors can extract a document with the given header.
    pub fn can_accept_any(&self, metadata: &HeaderMetadata) -> bool {
        self.extractors
            .iter()
            .any(|(classifier, _)| classifier.can_accept(metadata))
    }

    /// Sets up this object to extract a document with the given header.
    ///
    /// This function must be called before calling [Self::read].
    pub fn begin(&mut self, metadata: &HeaderMetadata) -> Result<(), crate::error::Error> {
        for (classifier, factory) in &self.extractors {
            if classifier.can_accept(metadata) {
                let extractor = factory(self.source.take().unwrap())?;

                self.extractor = Some(extractor);

                return Ok(());
            }
        }

        Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "no extractor").into())
    }

    /// Finishes extraction and returns the wrapped stream.
    pub fn finish(mut self) -> Result<S, crate::error::Error> {
        match self.extractor {
            Some(extractor) => extractor.finish_box(),
            None => Ok(self.source.take().unwrap()),
        }
    }
}

impl<'a, S: Read> Read for ExtractorDispatcher<'a, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.extractor.as_mut().unwrap().read(buf)
    }
}

/// Checks for WARC "resource" records.
pub struct ResourceClassifier;

impl Classifier for ResourceClassifier {
    fn can_accept(&self, metadata: &HeaderMetadata) -> bool {
        let warc_type = metadata
            .fields()
            .get_required("WARC-Type")
            .unwrap_or_default();

        warc_type == "resource"
    }
}

/// Extracts from WARC "resource" records.
pub struct ResourceExtractor<S: Read> {
    source: S,
}

impl<S: Read> ResourceExtractor<S> {
    /// Creates a ResourceExtractor with the given input stream.
    pub fn new(source: S) -> Result<Self, crate::error::Error>
    where
        Self: std::marker::Sized,
    {
        Ok(Self { source })
    }
}

impl<S: Read> Read for ResourceExtractor<S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.source.read(buf)
    }
}

impl<S: Read> Extractor<S> for ResourceExtractor<S> {
    fn get_ref(&self) -> &S {
        &self.source
    }

    fn get_mut(&mut self) -> &mut S {
        &mut self.source
    }

    fn into_inner(self) -> S {
        self.source
    }

    fn into_inner_box(self: Box<Self>) -> S {
        self.source
    }

    fn finish(self) -> Result<S, crate::error::Error> {
        Ok(self.source)
    }

    fn finish_box(self: Box<Self>) -> Result<S, crate::error::Error> {
        Ok(self.source)
    }
}

/// Checks for WARC "response" records with media type "application/http".
pub struct HTTPClassifier;

impl Classifier for HTTPClassifier {
    fn can_accept(&self, metadata: &HeaderMetadata) -> bool {
        let warc_type = match metadata.fields().get_required("WARC-Type") {
            Ok(warc_type) => warc_type,
            Err(_) => return false,
        };

        let content_type = match metadata
            .fields()
            .get_parsed_required::<MediaType>("Content-Type")
        {
            Ok(content_type) => content_type,
            Err(_) => return false,
        };

        warc_type == "response"
            && content_type.type_ == "application"
            && content_type.subtype == "http"
    }
}

/// Extracts from WARC "response" records with media type "application/http".
pub struct HTTPExtractor<'a, S: Read> {
    reader: MessageReader<'a, ComboReader<S>>,
}

impl<'a, S: Read> HTTPExtractor<'a, S> {
    /// Creates a new `HTTPExtractor` with the given input stream.
    pub fn new(source: S) -> Result<Self, crate::error::Error> {
        let mut reader = MessageReader::new(ComboReader::new(source));
        reader.begin_response(None)?;

        Ok(Self { reader })
    }
}

impl<'a, S: Read> Extractor<S> for HTTPExtractor<'a, S> {
    fn get_ref(&self) -> &S {
        self.reader.get_ref().get_ref()
    }

    fn get_mut(&mut self) -> &mut S {
        self.reader.get_mut().get_mut()
    }

    fn into_inner(self) -> S {
        self.reader.into_inner().into_inner()
    }

    fn into_inner_box(self: Box<Self>) -> S {
        self.reader.into_inner().into_inner()
    }

    fn finish(mut self) -> Result<S, crate::error::Error> {
        self.reader.end_message()?;
        Ok(self.reader.into_inner().into_inner())
    }

    fn finish_box(mut self: Box<Self>) -> Result<S, crate::error::Error> {
        self.reader.end_message()?;
        Ok(self.reader.into_inner().into_inner())
    }
}

impl<'a, S: Read> Read for HTTPExtractor<'a, S> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read_body().read(buf)
    }
}

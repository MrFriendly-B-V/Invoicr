use genpdf::{Element, RenderResult, Position, Size};
use genpdf::render::{Area, Layer};
use genpdf::fonts::FontCache;
use genpdf::style::Style;
use genpdf::error::{Error, ErrorKind};
use std::io::prelude::*;
use std::fs::File;
use std::io::Cursor;
use printpdf::image::codecs::png::PngDecoder;

pub struct Image {
    path:       String,
    image_type: ImageType
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct OwnedArea<'a> {
    pub layer: &'a Layer,
    pub origin: Position,
    pub size: Size
}

impl From<Area<'_>> for OwnedArea<'_> {
    fn from(a: Area) -> Self {
        unsafe {
            std::mem::transmute(a)
        }
    }
}

#[allow(dead_code)]
pub struct OwnedLayer {
    layer: printpdf::PdfLayerReference,
    size: Size
}

impl<'a> OwnedLayer {
    fn from(a: &'a Layer) -> &'a Self {
        unsafe {
            std::mem::transmute(a)
        }
    }
}

impl Element for Image {
    fn render(&mut self, _font_cache: &FontCache, area: Area<'_>, _style: Style) -> Result<RenderResult, Error> {
        let image = File::open(&self.path);
        if image.is_err() {
            let err = image.err().unwrap();
            return Err(Error::new(&err.to_string(), ErrorKind::IoError(err)));
        }

        let mut buffer = Vec::new();
        let read_result = image.unwrap().read_to_end(&mut buffer);
        if read_result.is_err() {
            let err = read_result.err().unwrap();
            return Err(Error::new(&err.to_string(), ErrorKind::IoError(err)))
        }

        let mut reader = Cursor::new(buffer.as_slice());
        match self.image_type {
            ImageType::Png => {
                let decoder = PngDecoder::new(&mut reader);
                if decoder.is_err() {
                    return Err(Error::new(decoder.err().unwrap().to_string(), ErrorKind::InvalidData));
                }

                let image = printpdf::types::plugins::graphics::image::Image::try_from(decoder.unwrap());
                if image.is_err() {
                    return Err(Error::new(image.err().unwrap().to_string(), ErrorKind::InvalidData));
                }

                let area_owned = OwnedArea::from(area);
                let layer_owned = OwnedLayer::from(area_owned.layer);
                image.unwrap().add_to_layer(layer_owned.layer.clone(), None, None, None, None, None, None);

            }
        };

        Ok(RenderResult::default())
    }
}

impl Image {
    pub fn png(path: &str) -> Self {
        Self {
            path: path.to_string(),
            image_type: ImageType::Png
        }
    }
}

enum ImageType {
    Png,
}
use parley::*;
use std::sync::Arc;
use vello::{
    kurbo::Affine,
    peniko::color::palette,
    peniko::{Brush, Color, Fill},
    *,
};

use crate::config::Settings;

pub struct Shield {
    settings: Arc<Settings>,
    contexts: (LayoutContext<Brush>, FontContext),
}

impl Shield {
    pub fn new(settings: Arc<Settings>) -> Shield {
        let font_cx = FontContext::new();
        let layout_cx = LayoutContext::new();
        Shield {
            settings,
            contexts: (layout_cx, font_cx),
        }
    }

    pub fn scene(&mut self, volume: f32) -> Scene {
        let mut scene = Scene::new();
        let layout = self.layout(format!("{:.0}%", volume * 100.0));
        let size = self.settings.size();
        let radius = self.settings.radius();
        scene.fill(
            vello::peniko::Fill::NonZero,
            Affine::IDENTITY,
            self.settings.background_color(),
            None,
            &Rect::new(0.0, 0.0, size.0.into(), size.1.into()).to_rounded_rect(radius.clone()),
        );

        for line in layout.lines() {
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let style = glyph_run.style();
                let mut x = glyph_run.offset();
                let y = glyph_run.baseline() + 85.0;
                let run = glyph_run.run();
                let font = run.font();
                let font_size = run.font_size();
                let synthesis = run.synthesis();
                let glyph_xform = synthesis
                    .skew()
                    .map(|angle| Affine::skew(angle.to_radians().tan() as f64, 0.0));

                scene
                    .draw_glyphs(font)
                    .brush(&style.brush)
                    .hint(true)
                    .glyph_transform(glyph_xform)
                    .font_size(font_size)
                    .normalized_coords(run.normalized_coords())
                    .draw(
                        Fill::NonZero,
                        glyph_run.glyphs().map(|glyph| {
                            let gx = x + glyph.x;
                            let gy = y - glyph.y;
                            x += glyph.advance;
                            vello::Glyph {
                                id: glyph.id as u32,
                                x: gx,
                                y: gy,
                            }
                        }),
                    );
            }
        }
        scene
    }

    fn layout(&mut self, text: String) -> Layout<Brush> {
        let mut builder = self
            .contexts
            .0
            .ranged_builder(&mut self.contexts.1, &text, 1.0, true);

        builder.push_default(StyleProperty::FontStack(FontStack::Single(
            FontFamily::Generic(GenericFamily::UiMonospace),
        )));
        builder.push_default(StyleProperty::FontSize(112.0));
        builder.push_default(StyleProperty::FontWeight(FontWeight::NORMAL));
        builder.push_default(StyleProperty::Brush(
            self.settings.foreground_color().to_owned().into(),
        ));
        builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(12.0)));

        // Build the builder into a Layout
        let mut layout = builder.build(&text);
        layout.break_all_lines(None);
        layout.align(Some(300.0), Alignment::Middle, AlignmentOptions::default());

        layout
    }
}

mod support;
use support::Run;
use vexel_scene::{
    Diagnostic, ErrorKind, ImagePixels, LimitKind, MediaError, MediaProvider, TextCommand,
    TextLayout, TextRequest, TextStyle, TextVerb, WorkControl,
};
use vexel_svg_media::{
    LayoutResult, MAX_PATH_COMMANDS, TEXT_CACHE_BYTES, TextCache, layout_shared_cache,
};

#[derive(Default)]
struct SizedProvider {
    calls: usize,
    commands: usize,
}
impl MediaProvider for SizedProvider {
    fn layout_text(
        &mut self,
        _: TextRequest<'_>,
        _: &mut dyn WorkControl,
    ) -> Result<TextLayout, MediaError> {
        self.calls += 1;
        Ok(TextLayout {
            glyphs: Vec::new(),
            commands: vec![
                TextCommand {
                    verb: TextVerb::Close,
                    values: [0.; 6],
                };
                self.commands
            ],
            advance_width: 1.,
            ascent: 0.,
            descent: 0.,
            leading: 0.,
        })
    }
    fn decode_image(
        &mut self,
        _: &[u8],
        _: &mut dyn WorkControl,
    ) -> Result<ImagePixels, MediaError> {
        Err(MediaError::Unsupported)
    }
}
enum Cache {
    Local(TextCache),
    Shared(vexel_scene::TextCache),
}
impl Cache {
    fn both() -> [Self; 2] {
        [
            Self::Local(TextCache::new()),
            Self::Shared(vexel_scene::TextCache::default()),
        ]
    }
    fn layout<'a>(
        &'a mut self,
        request: TextRequest<'_>,
        provider: &mut SizedProvider,
    ) -> Result<LayoutResult<'a>, Diagnostic> {
        match self {
            Self::Local(cache) => cache.layout(request, provider, &mut Run, 37),
            Self::Shared(cache) => layout_shared_cache(cache, request, &[], provider, &mut Run, 37),
        }
    }
    fn stats(&self) -> (usize, usize, usize) {
        match self {
            Self::Local(c) => (c.len(), c.stored_text_bytes(), c.stored_commands()),
            Self::Shared(c) => (c.entries.len(), c.text_bytes, c.commands),
        }
    }
}
fn request(text: &[u8]) -> TextRequest<'_> {
    TextRequest {
        text,
        font_families: &[],
        size_points: 9.,
        weight: 400.,
        style: TextStyle::Normal,
        letter_spacing_points: 0.,
    }
}

#[test]
fn local_and_accepted_shared_cache_admit_exactly_256_kib_then_return_owned_misses() {
    for mut cache in Cache::both() {
        let mut provider = SizedProvider::default();
        for byte in b'A'..=b'D' {
            let key = vec![byte; 64 * 1024];
            assert!(
                cache
                    .layout(request(&key), &mut provider)
                    .unwrap()
                    .is_cached()
            );
        }
        assert_eq!(cache.stats(), (4, TEXT_CACHE_BYTES, 0));
        let old_key = vec![b'A'; 64 * 1024];
        assert!(
            cache
                .layout(request(&old_key), &mut provider)
                .unwrap()
                .is_cached()
        );
        assert_eq!(provider.calls, 4, "owned key survives the original buffers");
        for _ in 0..2 {
            assert!(
                !cache
                    .layout(request(b"E"), &mut provider)
                    .unwrap()
                    .is_cached()
            );
        }
        assert_eq!(
            provider.calls, 6,
            "uncached valid layouts remain successful owned results"
        );
        assert_eq!(cache.stats(), (4, TEXT_CACHE_BYTES, 0));
    }
}

#[test]
fn local_and_accepted_shared_cache_enforce_cumulative_and_per_layout_command_caps() {
    for mut cache in Cache::both() {
        let mut provider = SizedProvider {
            commands: MAX_PATH_COMMANDS - 1,
            ..Default::default()
        };
        assert!(
            cache
                .layout(request(b"A"), &mut provider)
                .unwrap()
                .is_cached()
        );
        provider.commands = 1;
        assert!(
            cache
                .layout(request(b"B"), &mut provider)
                .unwrap()
                .is_cached()
        );
        assert_eq!(cache.stats(), (2, 2, MAX_PATH_COMMANDS));
        for _ in 0..2 {
            assert!(
                !cache
                    .layout(request(b"C"), &mut provider)
                    .unwrap()
                    .is_cached()
            );
        }
        assert_eq!(cache.stats(), (2, 2, MAX_PATH_COMMANDS));
        assert_eq!(provider.calls, 4);
        provider.commands = MAX_PATH_COMMANDS + 1;
        let error = match cache.layout(request(b"D"), &mut provider) {
            Err(error) => error,
            Ok(_) => panic!("oversized provider output must fail before cache admission"),
        };
        assert_eq!(
            (error.kind, error.offset),
            (ErrorKind::Limit(LimitKind::TextCommands), 37)
        );
        assert_eq!(cache.stats(), (2, 2, MAX_PATH_COMMANDS));
        assert!(
            cache
                .layout(request(b"B"), &mut provider)
                .unwrap()
                .is_cached()
        );
        assert_eq!(
            provider.calls, 5,
            "oversized output does not damage an earlier cache hit"
        );
    }
}

#[test]
fn cache_key_distinguishes_owned_family_and_each_font_parameter() {
    for mut cache in Cache::both() {
        let mut provider = SizedProvider::default();
        let family_a: [&[u8]; 1] = [b"Mock Sans"];
        let family_b: [&[u8]; 1] = [b"Other Sans"];
        let mut requests = vec![request(b"A"); 6];
        requests[1].font_families = &family_a;
        requests[2].font_families = &family_b;
        requests[3].weight = 700.;
        requests[4].style = TextStyle::Italic;
        requests[5].letter_spacing_points = 1.5;
        for r in &requests {
            assert!(cache.layout(*r, &mut provider).unwrap().is_cached());
        }
        let mut size = request(b"A");
        size.size_points = 12.;
        assert!(cache.layout(size, &mut provider).unwrap().is_cached());
        for r in &requests {
            assert!(cache.layout(*r, &mut provider).unwrap().is_cached());
        }
        assert_eq!(provider.calls, 7);
        assert_eq!(cache.stats().0, 7);
    }
}

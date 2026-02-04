// Glyph caching for efficient text rendering

use crate::metrics::CacheMetrics;
use fontdue::Font;
use std::collections::HashMap;

const MAX_CACHE_SIZE: usize = 512;

pub struct GlyphCache {
    cache: HashMap<GlyphKey, RasterizedGlyph>,
    metrics: CacheMetrics,
}

#[derive(Hash, Eq, PartialEq, Clone)]
struct GlyphKey {
    character: char,
    size_px: u32, // size in pixels * 10 for sub-pixel precision
}

pub struct RasterizedGlyph {
    pub bitmap: Vec<u8>,
    pub width: usize,
    pub height: usize,
    pub advance_width: f32,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::with_capacity(MAX_CACHE_SIZE),
            metrics: CacheMetrics::new(),
        }
    }

    pub fn get_or_rasterize(&mut self, font: &Font, c: char, size: f32) -> &RasterizedGlyph {
        let key = GlyphKey {
            character: c,
            size_px: (size * 10.0) as u32,
        };

        // Check if we have a cache hit
        if self.cache.contains_key(&key) {
            self.metrics.record_hit();
        } else {
            self.metrics.record_miss();
            // Insert new glyph
            let (metrics, bitmap) = font.rasterize(c, size);
            self.cache.insert(
                key.clone(),
                RasterizedGlyph {
                    bitmap,
                    width: metrics.width,
                    height: metrics.height,
                    advance_width: metrics.advance_width,
                },
            );
        }

        self.cache.get(&key).expect("Glyph must exist after insertion")
    }

    pub fn clear_if_full(&mut self) {
        if self.cache.len() >= MAX_CACHE_SIZE {
            let eviction_count = self.cache.len() as u64;
            self.cache.clear();
            self.metrics.record_eviction(eviction_count);
            tracing::debug!(
                evicted = eviction_count,
                hit_rate_pct = %self.metrics.hit_rate(),
                "Cleared glyph cache"
            );
        }
    }

    /// Get current cache metrics
    pub fn metrics(&self) -> &CacheMetrics {
        &self.metrics
    }

    /// Get current cache size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for GlyphCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontdue::{Font, FontSettings};

    fn get_test_font() -> Font {
        // Use fc-match to find a font dynamically
        use std::process::Command;

        if let Ok(output) = Command::new("fc-match")
            .args(["--format=%{file}", "sans"])
            .output()
        {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout);
                let path = path.trim();
                if !path.is_empty() {
                    if let Ok(font_data) = std::fs::read(path) {
                        if let Ok(font) = Font::from_bytes(font_data, FontSettings::default()) {
                            return font;
                        }
                    }
                }
            }
        }

        panic!("No test font available - install fonts or ensure fontconfig is available");
    }

    #[test]
    fn test_glyph_cache() {
        let font = get_test_font();
        let mut cache = GlyphCache::new();

        // First access - rasterize glyph (cache miss)
        let width1 = {
            let glyph1 = cache.get_or_rasterize(&font, 'A', 16.0);
            assert!(glyph1.width > 0);
            assert!(glyph1.height > 0);
            glyph1.width
        };

        // Should be a miss
        assert_eq!(cache.metrics().misses(), 1);
        assert_eq!(cache.metrics().hits(), 0);

        // Second access should use cache (cache hit)
        let width2 = cache.get_or_rasterize(&font, 'A', 16.0).width;
        assert_eq!(width1, width2);

        // Should now have a hit
        assert_eq!(cache.metrics().hits(), 1);
        assert_eq!(cache.metrics().misses(), 1);
        assert_eq!(cache.metrics().hit_rate(), 50.0);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = GlyphCache::new();
        assert_eq!(cache.len(), 0);

        // Fill cache beyond limit
        cache.cache.insert(
            GlyphKey {
                character: 'A',
                size_px: 160,
            },
            RasterizedGlyph {
                bitmap: vec![],
                width: 10,
                height: 10,
                advance_width: 10.0,
            },
        );

        cache.clear_if_full();
        // Should only clear if at MAX_CACHE_SIZE
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_metrics_hit_rate() {
        let font = get_test_font();
        let mut cache = GlyphCache::new();

        // Access same glyph multiple times
        for _ in 0..10 {
            cache.get_or_rasterize(&font, 'X', 16.0);
        }

        // First was miss, rest are hits
        assert_eq!(cache.metrics().misses(), 1);
        assert_eq!(cache.metrics().hits(), 9);
        assert_eq!(cache.metrics().hit_rate(), 90.0);
    }
}

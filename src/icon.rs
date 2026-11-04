//! 程序图标：用代码画一只顶着黑眼圈的牛，不依赖图片文件。

const ICO_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];

#[derive(Clone, Copy)]
struct Rgba(f32, f32, f32, f32);

const HORN: Rgba = Rgba(0.93, 0.80, 0.55, 1.0);
const EAR: Rgba = Rgba(0.82, 0.55, 0.35, 1.0);
const HEAD: Rgba = Rgba(0.99, 0.96, 0.90, 1.0);
const SPOT: Rgba = Rgba(0.17, 0.15, 0.13, 1.0);
const MUZZLE: Rgba = Rgba(0.97, 0.68, 0.62, 1.0);
const EYE_BAG: Rgba = Rgba(0.60, 0.55, 0.62, 1.0);

/// 椭圆内部 1，外部 0，边缘按像素宽度羽化。
fn ellipse(x: f32, y: f32, cx: f32, cy: f32, rx: f32, ry: f32, px: f32) -> f32 {
    let dist = (((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2)).sqrt();
    let edge = (dist - 1.0) * rx.min(ry);
    (0.5 - edge / px).clamp(0.0, 1.0)
}

fn over(dst: Rgba, color: Rgba, cover: f32) -> Rgba {
    let a = color.3 * cover;
    let out_a = a + dst.3 * (1.0 - a);
    if out_a <= 0.0 {
        return Rgba(0.0, 0.0, 0.0, 0.0);
    }
    let mix = |s: f32, d: f32| (s * a + d * dst.3 * (1.0 - a)) / out_a;
    Rgba(mix(color.0, dst.0), mix(color.1, dst.1), mix(color.2, dst.2), out_a)
}

fn pixel(x: f32, y: f32, px: f32) -> Rgba {
    let mut c = Rgba(0.0, 0.0, 0.0, 0.0);
    for side in [-1.0f32, 1.0] {
        c = over(c, HORN, ellipse(x, y, side * 0.56, -0.74, 0.17, 0.25, px));
        c = over(c, EAR, ellipse(x, y, side * 0.80, -0.26, 0.23, 0.13, px));
    }
    let head = ellipse(x, y, 0.0, -0.04, 0.68, 0.80, px);
    c = over(c, HEAD, head);
    // 花斑限制在头部以内。
    c = over(c, SPOT, ellipse(x, y, -0.36, -0.46, 0.30, 0.32, px).min(head));
    for side in [-1.0f32, 1.0] {
        c = over(c, EYE_BAG, ellipse(x, y, side * 0.27, -0.09, 0.15, 0.15, px));
        c = over(c, SPOT, ellipse(x, y, side * 0.27, -0.13, 0.10, 0.11, px));
    }
    c = over(c, MUZZLE, ellipse(x, y, 0.0, 0.42, 0.56, 0.36, px).min(head));
    for side in [-1.0f32, 1.0] {
        c = over(c, SPOT, ellipse(x, y, side * 0.20, 0.38, 0.07, 0.09, px));
    }
    c
}

/// 生成 RGBA 图像。
pub fn rgba(size: u32) -> Vec<u8> {
    const SUPER: u32 = 3;
    let px = 2.0 / size as f32;
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for j in 0..size {
        for i in 0..size {
            let (mut r, mut g, mut b, mut a) = (0.0, 0.0, 0.0, 0.0);
            for sj in 0..SUPER {
                for si in 0..SUPER {
                    let x = (i as f32 + (si as f32 + 0.5) / SUPER as f32) * px - 1.0;
                    let y = (j as f32 + (sj as f32 + 0.5) / SUPER as f32) * px - 1.0;
                    let p = pixel(x, y, px / SUPER as f32);
                    r += p.0 * p.3;
                    g += p.1 * p.3;
                    b += p.2 * p.3;
                    a += p.3;
                }
            }
            let (out_r, out_g, out_b) = if a > 0.0 { (r / a, g / a, b / a) } else { (0.0, 0.0, 0.0) };
            let n = (SUPER * SUPER) as f32;
            for value in [out_r, out_g, out_b, a / n] {
                out.push((value * 255.0).round().clamp(0.0, 255.0) as u8);
            }
        }
    }
    out
}

/// ICO 内嵌的 DIB：BGRA 自下而上，后接 AND 掩码。
fn dib(size: u32) -> Vec<u8> {
    let pixels = rgba(size);
    let mask_stride = size.div_ceil(32) * 4;
    let mut out = Vec::new();
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&(size * 2).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 24]);
    for row in (0..size).rev() {
        for col in 0..size {
            let i = ((row * size + col) * 4) as usize;
            out.extend_from_slice(&[pixels[i + 2], pixels[i + 1], pixels[i], pixels[i + 3]]);
        }
    }
    out.extend(std::iter::repeat_n(0u8, (mask_stride * size) as usize));
    out
}

pub fn ico_images() -> Vec<(u32, Vec<u8>)> {
    ICO_SIZES.iter().map(|&s| (s, dib(s))).collect()
}

/// 完整的 .ico 文件内容，给系统通知用。
pub fn ico_file() -> Vec<u8> {
    let images = ico_images();
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, bytes) in &images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += bytes.len() as u32;
    }
    for (_, bytes) in &images {
        out.extend_from_slice(bytes);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_has_opaque_center_and_clear_corner() {
        let size = 32;
        let img = rgba(size);
        assert_eq!(img.len(), (size * size * 4) as usize);
        assert_eq!(img[3], 0, "角落透明");
        let center = ((size / 2 * size + size / 2) * 4 + 3) as usize;
        assert_eq!(img[center], 255, "中心不透明");
    }

    #[test]
    fn ico_header_matches_image_table() {
        let file = ico_file();
        assert_eq!(&file[..4], &[0, 0, 1, 0]);
        assert_eq!(u16::from_le_bytes([file[4], file[5]]) as usize, ICO_SIZES.len());
        let last = 6 + 16 * (ICO_SIZES.len() - 1);
        let len = u32::from_le_bytes(file[last + 8..last + 12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(file[last + 12..last + 16].try_into().unwrap()) as usize;
        assert_eq!(offset + len, file.len());
    }
}

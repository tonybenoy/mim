use image::{DynamicImage, GenericImageView, Rgb, RgbImage};
use ndarray::Array4;

/// Resize image with letterboxing to maintain aspect ratio.
/// Returns (resized image, scale_x, scale_y, pad_x, pad_y).
pub fn letterbox_resize(
    img: &DynamicImage,
    target_w: u32,
    target_h: u32,
) -> (RgbImage, f32, f32, f32, f32) {
    let (orig_w, orig_h) = img.dimensions();
    let scale = f32::min(
        target_w as f32 / orig_w as f32,
        target_h as f32 / orig_h as f32,
    );

    let new_w = (orig_w as f32 * scale) as u32;
    let new_h = (orig_h as f32 * scale) as u32;

    let resized = image::imageops::resize(
        &img.to_rgb8(),
        new_w,
        new_h,
        image::imageops::FilterType::Triangle,
    );

    let mut canvas = RgbImage::new(target_w, target_h);
    let pad_x = (target_w - new_w) / 2;
    let pad_y = (target_h - new_h) / 2;

    image::imageops::overlay(&mut canvas, &resized, pad_x as i64, pad_y as i64);

    (canvas, scale, scale, pad_x as f32, pad_y as f32)
}

/// Convert an RGB image to a normalized NCHW f32 tensor.
pub fn image_to_tensor(
    img: &RgbImage,
    mean: [f32; 3],
    std: [f32; 3],
) -> Array4<f32> {
    let (w, h) = img.dimensions();
    let mut tensor = Array4::<f32>::zeros((1, 3, h as usize, w as usize));

    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);
            for c in 0..3 {
                tensor[[0, c, y as usize, x as usize]] =
                    (pixel[c] as f32 - mean[c]) / std[c];
            }
        }
    }

    tensor
}

/// Standard ArcFace reference landmarks for 112x112 alignment.
const ARCFACE_REF: [[f32; 2]; 5] = [
    [38.2946, 51.6963],  // left eye
    [73.5318, 51.5014],  // right eye
    [56.0252, 71.7366],  // nose tip
    [41.5493, 92.3655],  // left mouth
    [70.7299, 92.2041],  // right mouth
];

/// Compute a similarity transform (rotation + uniform scale + translation)
/// from source points to destination points using least-squares.
/// Returns the 2x3 affine matrix.
fn estimate_similarity_transform(src: &[[f32; 2]; 5], dst: &[[f32; 2]; 5]) -> [[f32; 3]; 2] {
    // Compute centroids
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    let (mut dx, mut dy) = (0.0f32, 0.0f32);
    for i in 0..5 {
        sx += src[i][0];
        sy += src[i][1];
        dx += dst[i][0];
        dy += dst[i][1];
    }
    sx /= 5.0;
    sy /= 5.0;
    dx /= 5.0;
    dy /= 5.0;

    // Compute scale and rotation via Procrustes
    let mut num_r = 0.0f32; // sum of cross terms for rotation angle
    let mut den_r = 0.0f32; // sum of cross terms for rotation angle
    let mut src_var = 0.0f32;

    for i in 0..5 {
        let sxi = src[i][0] - sx;
        let syi = src[i][1] - sy;
        let dxi = dst[i][0] - dx;
        let dyi = dst[i][1] - dy;

        num_r += sxi * dyi - syi * dxi;
        den_r += sxi * dxi + syi * dyi;
        src_var += sxi * sxi + syi * syi;
    }

    let angle = num_r.atan2(den_r);
    let scale = (num_r * num_r + den_r * den_r).sqrt() / src_var;

    let cos_a = scale * angle.cos();
    let sin_a = scale * angle.sin();

    let tx = dx - cos_a * sx + sin_a * sy;
    let ty = dy - sin_a * sx - cos_a * sy;

    [
        [cos_a, -sin_a, tx],
        [sin_a, cos_a, ty],
    ]
}

/// Align a face crop using 5 detected landmarks to the canonical ArcFace reference.
/// Output is a 112x112 RGB image.
pub fn align_face(img: &DynamicImage, landmarks: &[f32]) -> RgbImage {
    let output_size = 112u32;

    // Parse landmarks: [x0,y0, x1,y1, x2,y2, x3,y3, x4,y4]
    let src: [[f32; 2]; 5] = [
        [landmarks[0], landmarks[1]],
        [landmarks[2], landmarks[3]],
        [landmarks[4], landmarks[5]],
        [landmarks[6], landmarks[7]],
        [landmarks[8], landmarks[9]],
    ];

    let affine = estimate_similarity_transform(&src, &ARCFACE_REF);
    let rgb = img.to_rgb8();
    let (src_w, src_h) = rgb.dimensions();

    let mut output = RgbImage::new(output_size, output_size);

    // Apply inverse transform to map output pixels to source pixels
    let det = affine[0][0] * affine[1][1] - affine[0][1] * affine[1][0];
    if det.abs() < 1e-8 {
        // Degenerate transform — fallback to center crop + resize
        let cropped = img.resize_exact(output_size, output_size, image::imageops::FilterType::Triangle);
        return cropped.to_rgb8();
    }

    let inv_det = 1.0 / det;
    let inv = [
        [affine[1][1] * inv_det, -affine[0][1] * inv_det],
        [-affine[1][0] * inv_det, affine[0][0] * inv_det],
    ];
    let inv_tx = -(inv[0][0] * affine[0][2] + inv[0][1] * affine[1][2]);
    let inv_ty = -(inv[1][0] * affine[0][2] + inv[1][1] * affine[1][2]);

    for oy in 0..output_size {
        for ox in 0..output_size {
            let fx = ox as f32 + 0.5;
            let fy = oy as f32 + 0.5;

            let src_x = inv[0][0] * fx + inv[0][1] * fy + inv_tx;
            let src_y = inv[1][0] * fx + inv[1][1] * fy + inv_ty;

            // Bilinear interpolation
            let x0 = src_x.floor() as i32;
            let y0 = src_y.floor() as i32;
            let x1 = x0 + 1;
            let y1 = y0 + 1;

            let wx = src_x - x0 as f32;
            let wy = src_y - y0 as f32;

            let get_pixel = |x: i32, y: i32| -> [f32; 3] {
                if x >= 0 && x < src_w as i32 && y >= 0 && y < src_h as i32 {
                    let p = rgb.get_pixel(x as u32, y as u32);
                    [p[0] as f32, p[1] as f32, p[2] as f32]
                } else {
                    [0.0, 0.0, 0.0]
                }
            };

            let p00 = get_pixel(x0, y0);
            let p10 = get_pixel(x1, y0);
            let p01 = get_pixel(x0, y1);
            let p11 = get_pixel(x1, y1);

            let mut rgb_out = [0u8; 3];
            for c in 0..3 {
                let val = p00[c] * (1.0 - wx) * (1.0 - wy)
                    + p10[c] * wx * (1.0 - wy)
                    + p01[c] * (1.0 - wx) * wy
                    + p11[c] * wx * wy;
                rgb_out[c] = val.clamp(0.0, 255.0) as u8;
            }

            output.put_pixel(ox, oy, Rgb(rgb_out));
        }
    }

    output
}

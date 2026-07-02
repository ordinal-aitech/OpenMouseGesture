// 概要: mouse-actionsアルゴリズムによるマウスジェスチャー認識
// 入力: recognize(points, templates) にて、マウス軌跡 points と
//       テンプレート配列 templates を受け取る
// 出力: 認識されたジェスチャー名 Option<String> と信頼度
// 例:
//   - 軌跡を正規化し、角度配列に変換
//   - 移動平均フィルタでスムージング
//   - オフセット補正付きパターンマッチングで認識

use crate::config::GestureTemplate;
use std::f64::consts::PI;

const NORMALIZE_SIZE: i32 = 1000;
const MOV_AVG_COEFFICIENT: f64 = 0.8;
const SHAPE_MIN_SIZE: usize = 8;
const DIFF_MAX: f64 = 0.8;
const DIFF_MIN_WITH_SECOND: f64 = 0.05;
const MAX_OFFSET: usize = 20;

#[derive(Debug, Clone)]
struct Point {
    x: i32,
    y: i32,
}

pub fn recognize(points: &[(f64, f64)], templates: &[GestureTemplate]) -> Option<String> {
    if points.len() < 5 {
        return None;
    }
    if templates.is_empty() {
        return None;
    }

    let int_points: Vec<Point> = points
        .iter()
        .map(|(x, y)| Point {
            x: *x as i32,
            y: *y as i32,
        })
        .collect();

    let normalized = normalize_points(&int_points);
    let angles = points_to_angles(&normalized);

    if angles.len() < SHAPE_MIN_SIZE {
        return None;
    }

    let mut candidates: Vec<(String, f64)> = Vec::new();

    for template in templates {
        let template_points: Vec<Point> = template
            .points
            .iter()
            .map(|(x, y)| Point {
                x: *x as i32,
                y: *y as i32,
            })
            .collect();

        let template_normalized = normalize_points(&template_points);
        let template_angles = points_to_angles(&template_normalized);

        if template_angles.len() < SHAPE_MIN_SIZE {
            continue;
        }

        let diff = compare_angles_with_offset(&angles, &template_angles);
        candidates.push((template.name.clone(), diff));
    }

    if candidates.is_empty() {
        return None;
    }

    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let first_diff = candidates[0].1;
    if first_diff >= DIFF_MAX {
        return None;
    }

    if candidates.len() > 1 {
        let second_diff = candidates[1].1;
        if (second_diff - first_diff) < DIFF_MIN_WITH_SECOND {
            return None;
        }
    }

    let confidence = (100.0 - first_diff * first_diff * 100.0).max(0.0);
    eprintln!(
        "[GESTURE_RECOGNIZER] Recognized: {} (confidence: {:.2}%, diff: {:.2})",
        candidates[0].0, confidence, first_diff
    );

    Some(candidates[0].0.clone())
}

fn normalize_points(points: &[Point]) -> Vec<Point> {
    if points.is_empty() {
        return Vec::new();
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for p in points {
        min_x = min_x.min(p.x);
        max_x = max_x.max(p.x);
        min_y = min_y.min(p.y);
        max_y = max_y.max(p.y);
    }

    let width = max_x - min_x;
    let height = max_y - min_y;
    let size = width.max(height);

    if size == 0 {
        return points.to_vec();
    }

    points
        .iter()
        .map(|p| Point {
            x: (NORMALIZE_SIZE * (p.x - min_x)) / size,
            y: (NORMALIZE_SIZE * (p.y - min_y)) / size,
        })
        .collect()
}

fn points_to_angles(points: &[Point]) -> Vec<f64> {
    if points.len() < 2 {
        return Vec::new();
    }

    let mut angles = Vec::new();
    let mut rx = 0.0;
    let mut ry = 0.0;
    let mut last = &points[0];

    for current in &points[1..] {
        let dx = (current.x - last.x) as f64;
        let dy = (current.y - last.y) as f64;

        rx = MOV_AVG_COEFFICIENT * rx + (1.0 - MOV_AVG_COEFFICIENT) * dx;
        ry = MOV_AVG_COEFFICIENT * ry + (1.0 - MOV_AVG_COEFFICIENT) * dy;

        let hyp = (rx * rx + ry * ry).sqrt();
        if hyp > 0.0 {
            let angle = if ry <= 0.0 {
                (rx / hyp).acos()
            } else {
                -(rx / hyp).acos()
            };
            angles.push(angle);
        }

        last = current;
    }

    angles
}

fn compare_angles(angles_a: &[f64], angles_b: &[f64]) -> f64 {
    if angles_a.is_empty() || angles_b.is_empty() {
        return f64::MAX;
    }

    let (smaller_vec, bigger_vec) = if angles_a.len() <= angles_b.len() {
        (angles_a, angles_b)
    } else {
        (angles_b, angles_a)
    };

    let mut total_diff = 0.0;

    for (i, &angle_a) in smaller_vec.iter().enumerate() {
        let progress = i as f64 / smaller_vec.len() as f64;
        let b_index = (bigger_vec.len() as f64 * progress) as usize;
        let b_index = b_index.min(bigger_vec.len() - 1);
        let angle_b = bigger_vec[b_index];

        let raw_diff = (angle_a - angle_b).abs();
        let diff = if raw_diff > PI {
            2.0 * PI - raw_diff
        } else {
            raw_diff
        };
        total_diff += diff;
    }

    total_diff / smaller_vec.len() as f64
}

fn compare_angles_with_offset(angles1: &[f64], angles2: &[f64]) -> f64 {
    let mut min_diff = compare_angles(angles1, angles2);

    let shorter_len = angles1.len().min(angles2.len());
    let offset_max = (shorter_len / 10).min(MAX_OFFSET);

    if offset_max < 2 {
        return min_diff;
    }

    for i in (2..offset_max).step_by(2) {
        if i < angles1.len() {
            let diff1 = compare_angles(&angles1[i..], angles2);
            min_diff = min_diff.min(diff1);

            let end = angles1.len() - i;
            if end > 0 {
                let diff2 = compare_angles(&angles1[0..end], angles2);
                min_diff = min_diff.min(diff2);
            }
        }

        if i < angles2.len() {
            let diff3 = compare_angles(angles1, &angles2[i..]);
            min_diff = min_diff.min(diff3);

            let end = angles2.len() - i;
            if end > 0 {
                let diff4 = compare_angles(angles1, &angles2[0..end]);
                min_diff = min_diff.min(diff4);
            }
        }
    }

    min_diff
}

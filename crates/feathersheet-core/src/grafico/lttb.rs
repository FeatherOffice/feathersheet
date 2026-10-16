//! Algoritmo LTTB (Largest-Triangle-Three-Buckets) em Rust para redução de densidade de pontos em gráficos.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn downsample_lttb(data: &[Point], threshold: usize) -> Vec<Point> {
    let data_len = data.len();
    if threshold >= data_len || threshold <= 2 {
        return data.to_vec();
    }

    let mut sampled = Vec::with_capacity(threshold);
    let every = (data_len - 2) as f64 / (threshold - 2) as f64;
    let mut a = 0;

    sampled.push(data[a]);

    for i in 0..(threshold - 2) {
        let mut avg_x = 0.0;
        let mut avg_y = 0.0;
        let avg_range_start = ((i + 1) as f64 * every).floor() as usize + 1;
        let mut avg_range_end = ((i + 2) as f64 * every).floor() as usize + 1;
        if avg_range_end > data_len {
            avg_range_end = data_len;
        }

        let avg_range_len = avg_range_end.saturating_sub(avg_range_start);
        if avg_range_len > 0 {
            for j in avg_range_start..avg_range_end {
                avg_x += data[j].x;
                avg_y += data[j].y;
            }
            avg_x /= avg_range_len as f64;
            avg_y /= avg_range_len as f64;
        } else if avg_range_start < data_len {
            avg_x = data[avg_range_start].x;
            avg_y = data[avg_range_start].y;
        }

        let range_offs = ((i + 0) as f64 * every).floor() as usize + 1;
        let range_to = ((i + 1) as f64 * every).floor() as usize + 1;

        let point_a_x = data[a].x;
        let point_a_y = data[a].y;
        let mut max_area = -1.0;
        let mut max_area_idx = range_offs;

        for j in range_offs..range_to.min(data_len) {
            let pt = &data[j];
            let area = ((point_a_x - avg_x) * (pt.y - point_a_y)
                - (point_a_x - pt.x) * (avg_y - point_a_y))
                .abs()
                * 0.5;

            if area > max_area {
                max_area = area;
                max_area_idx = j;
            }
        }

        a = max_area_idx;
        sampled.push(data[a]);
    }

    sampled.push(data[data_len - 1]);
    sampled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lttb_downsample() {
        let points: Vec<Point> = (0..100)
            .map(|i| Point {
                x: i as f64,
                y: (i as f64 * 0.1).sin(),
            })
            .collect();

        let reduced = downsample_lttb(&points, 10);
        assert_eq!(reduced.len(), 10);
        assert_eq!(reduced[0], points[0]);
        assert_eq!(reduced[9], points[99]);
    }
}

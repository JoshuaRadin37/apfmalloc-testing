use std::borrow::Borrow;
use std::collections::HashMap;
use std::error::Error;
use std::iter::FromIterator;
use std::path::PathBuf;

use plotters::drawing::{BitMapBackend, DrawingBackend, IntoDrawingArea};
use plotters::palette::chromatic_adaptation::AdaptInto;
use plotters::prelude::{Color, LineSeries, RGBColor, ShapeStyle};
use plotters::prelude::backend::BackendStyle;
use plotters::style::{IntoFont, RGBAColor};
use plotters::style::text_anchor::{HPos, Pos, VPos};
use random_color::{Luminosity, RandomColor};

use crate::benchmark::Benchmark;

pub struct Graph<'a> {
    benchmark: String,
    results: HashMap<&'a str, Vec<f64>>,
    num_threads: usize
}

const GRAPHS_DIR: &str = "./graphs";

fn generate_graph_path(benchmark_name: &str) -> PathBuf {
    let graph_name = format!("{}.png", benchmark_name);
    std::fs::create_dir_all(GRAPHS_DIR);
    PathBuf::from_iter(&[GRAPHS_DIR, & *graph_name])
}

impl <'a> Graph<'a> {

    pub fn new(benchmark: String, results: HashMap<&'a str, Vec<f64>>, num_threads: usize) -> Self {
        Self {
            benchmark,
            results,
            num_threads
        }
    }

    fn get_line_series(&self, allocator: &&'a str) -> impl Iterator<Item=(usize, f64)> {
        let points =
            self.results[allocator]
                .iter()
                .enumerate()
                .map(|(index, throughput)| (index + 1, *throughput))
                .collect::<Vec<_>>()
                .into_iter();
        points
    }

    fn get_max_throughput(&self) -> f64 {
        let mut max = 0.0f64;
        for allocator in self.results.keys() {
            let results = &self.results[allocator];
            for throughput in results {
                max = max.max(*throughput);
            }
        }
        max
    }

    #[must_use]
    pub fn make_graph(self) -> Result<(), Box<dyn Error>> {
        use plotters::prelude::*;
        println!("Generating graph");
        let path = generate_graph_path(&*self.benchmark);
        let root = BitMapBackend::new(
            &path,
            (720, 600)
        ).into_drawing_area();
        let root = root.margin(10, 10, 10, 10);
        root.fill(&WHITE);

        let max_y: f64 = self.get_max_throughput() + 10.0;
        let labels = (max_y / 10.0) as usize;

        let mut chart = ChartBuilder::on(&root)
            .caption(format!("{} Throughput vs Number of Threads", self.benchmark), ("sans-serif", 30).into_font())
            .x_label_area_size(40)
            .margin_right(20)
            .y_label_area_size(60)
            .margin_bottom(10)
            .margin_left(10)
            .build_ranged(1..self.num_threads, 0f64..max_y).unwrap();

        chart
            .configure_mesh()
            .x_labels(self.num_threads)
            .y_labels(labels)
            .x_desc("Number of Threads")
            .y_desc("Throughput")
            .draw()?;

        let mut created_colors: Vec<(u8, u8, u8)> = vec![];

        for allocator in self.results.keys() {
            let random_color = RandomColor::new()
                .alpha(0.0)
                .to_rgb_array();
            let color = loop {
                let out = RGBColor(random_color[0] as u8, random_color[1] as u8, random_color[2] as u8);
                let RGBColor (r, g, b) = &out;
                if !created_colors.contains(&(*r, *g, *b)) {
                    break out;
                }
            };
            {
                let RGBColor (r, g, b) = &color;
                created_colors.push((*r, *g, *b));
            }

            let series = self.get_line_series(allocator);


            chart.draw_series(
                LineSeries::new(series,
                                {
                                    let mut ret = ShapeStyle::from(&color);
                                    ret.stroke_width = 3;
                                    ret
                                }
                )
            )?
                .label(format!("{}", allocator))
                .legend(move |(base_x, base_y)| {
                    let mut style = ShapeStyle::from(&color);
                    style.filled = true;
                    style.stroke_width = 10;
                    Rectangle::new(
                        [(base_x, base_y - 3), (base_x + 25, base_y + 3)],
                        style
                    )
                }
                );
        }

        chart.configure_series_labels()
            .position(
                SeriesLabelPosition::UpperRight
            )
            .border_style(
                ShapeStyle::from(&BLACK)
            )
            .background_style(
                ShapeStyle::from(&WHITE)
            )
            .draw()?;




        Ok(())
    }
}


//! The output columns of a query, in their order (E1-5 "Column order").

/// One output column: a dimension or a metric.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputColumn<Dimension, Metric> {
    /// A dimension.
    Dimension(Dimension),
    /// A metric.
    Metric(Metric),
}

/// The output columns of a query, in output order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Columns<Dimension, Metric>(Vec<OutputColumn<Dimension, Metric>>);

impl<Dimension, Metric> Columns<Dimension, Metric> {
    /// The columns of the wide-table form: the dimensions, then the metrics, each in the order
    /// written (E1-5).
    #[must_use]
    pub fn dimensions_then_metrics(dimensions: Vec<Dimension>, metrics: Vec<Metric>) -> Self {
        let dimensions = dimensions.into_iter().map(OutputColumn::Dimension);
        let metrics = metrics.into_iter().map(OutputColumn::Metric);
        Self(dimensions.chain(metrics).collect())
    }

    /// The columns of the SQL form, in `SELECT` order (E1-5).
    #[must_use]
    pub fn in_select_order(columns: Vec<OutputColumn<Dimension, Metric>>) -> Self {
        Self(columns)
    }

    /// Every column, in output order.
    pub fn iter(&self) -> impl Iterator<Item = &OutputColumn<Dimension, Metric>> {
        self.0.iter()
    }

    /// The column at `position`, from 0.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<&OutputColumn<Dimension, Metric>> {
        self.0.get(position)
    }

    /// The first column.
    #[must_use]
    pub fn first(&self) -> Option<&OutputColumn<Dimension, Metric>> {
        self.0.first()
    }

    /// The dimensions, in output order.
    pub fn dimensions(&self) -> impl Iterator<Item = &Dimension> {
        self.0.iter().filter_map(|column| match column {
            OutputColumn::Dimension(dimension) => Some(dimension),
            OutputColumn::Metric(_) => None,
        })
    }

    /// The metrics, in output order. A metric's position in this sequence identifies it in
    /// the branches (E3-4).
    pub fn metrics(&self) -> impl Iterator<Item = &Metric> {
        self.0.iter().filter_map(|column| match column {
            OutputColumn::Metric(metric) => Some(metric),
            OutputColumn::Dimension(_) => None,
        })
    }
}

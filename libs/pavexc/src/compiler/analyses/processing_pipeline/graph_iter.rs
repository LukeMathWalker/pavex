use crate::compiler::analyses::call_graph::OrderedCallGraph;
use crate::compiler::analyses::processing_pipeline::RequestHandlerPipeline;

/// See [`RequestHandlerPipeline::graph_iter`] for more information.
pub(crate) struct PipelineGraphIterator<'a> {
    pub(super) pipeline: &'a RequestHandlerPipeline,
    pub(super) current_stage: Option<usize>,
}

impl<'a> Iterator for PipelineGraphIterator<'a> {
    type Item = &'a OrderedCallGraph;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(stage) = self.current_stage else {
            return None;
        };
        let stage_data = self.pipeline.middleware_id2stage_data.get_index(stage);
        if let Some((_, stage_data)) = stage_data {
            self.current_stage = Some(stage + 1);
            Some(&stage_data.call_graph)
        } else {
            self.current_stage = None;
            Some(&self.pipeline.handler_call_graph)
        }
    }
}

impl<'a> ExactSizeIterator for PipelineGraphIterator<'a> {
    fn len(&self) -> usize {
        self.pipeline.middleware_id2stage_data.len() + 1
    }
}

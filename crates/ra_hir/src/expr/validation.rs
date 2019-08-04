use rustc_hash::FxHashSet;
use std::sync::Arc;

use ra_syntax::ast::{AstNode, StructLit};

use super::{Expr, ExprId, StructLitField, TypeRef};
use crate::{
    adt::AdtDef,
    diagnostics::{DiagnosticSink, MissingFields},
    expr::AstPtr,
    ty::InferenceResult,
    FnData, Function, HasSource, HirDatabase, Name, Path
};

pub(crate) struct ExprValidator<'a, 'b: 'a> {
    func: Function,
    infer: Arc<InferenceResult>,
    sink: &'a mut DiagnosticSink<'b>,
}

impl<'a, 'b> ExprValidator<'a, 'b> {
    pub(crate) fn new(
        func: Function,
        infer: Arc<InferenceResult>,
        sink: &'a mut DiagnosticSink<'b>,
    ) -> ExprValidator<'a, 'b> {
        ExprValidator { func, infer, sink }
    }

    pub(crate) fn validate_body(&mut self, db: &impl HirDatabase) {
        let body = self.func.body(db);
        let mut final_expr = None;
        for e in body.exprs() {
            final_expr = Some(e);
            if let (id, Expr::StructLit { path, fields, spread }) = e {
                self.validate_struct_literal(id, path, fields, *spread, db);
            }
        }
        if let Some(e) = final_expr {
            let fn_data = FnData::fn_data_query(
                db,
                self.func
            );
            let expr_ty = &self.infer[e.0];
            println!("FUNCTION RETURN TYPE: {:?}", fn_data.ret_type());
            println!("EXPR TYPE: {:?}", expr_ty);
        }
    }

    fn validate_struct_literal(
        &mut self,
        id: ExprId,
        _path: &Option<Path>,
        fields: &[StructLitField],
        spread: Option<ExprId>,
        db: &impl HirDatabase,
    ) {
        if spread.is_some() {
            return;
        }

        let struct_def = match self.infer[id].as_adt() {
            Some((AdtDef::Struct(s), _)) => s,
            _ => return,
        };

        let lit_fields: FxHashSet<_> = fields.iter().map(|f| &f.name).collect();
        let missed_fields: Vec<Name> = struct_def
            .fields(db)
            .iter()
            .filter_map(|f| {
                let name = f.name(db);
                if lit_fields.contains(&name) {
                    None
                } else {
                    Some(name)
                }
            })
            .collect();
        if missed_fields.is_empty() {
            return;
        }
        let source_map = self.func.body_source_map(db);
        let file_id = self.func.source(db).file_id;
        let parse = db.parse(file_id.original_file(db));
        let source_file = parse.tree();
        if let Some(field_list_node) = source_map
            .expr_syntax(id)
            .map(|ptr| ptr.to_node(source_file.syntax()))
            .and_then(StructLit::cast)
            .and_then(|lit| lit.named_field_list())
        {
            let field_list_ptr = AstPtr::new(&field_list_node);
            self.sink.push(MissingFields {
                file: file_id,
                field_list: field_list_ptr,
                missed_fields,
            })
        }
    }

    fn validate_results_in_tail_expr(
        &mut self,
        id: ExprId,
        _path: &Option<Path>,
        fields: &[StructLitField],
        spread: Option<ExprId>,
        db: &impl HirDatabase,
    ) {
        // TODO!
    }
}

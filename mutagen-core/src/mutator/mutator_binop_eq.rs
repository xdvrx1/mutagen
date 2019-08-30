//! Mutator for binary operations `==` and `!=`

use std::ops::Deref;

use proc_macro2::{Span, TokenStream};
use quote::quote_spanned;
use quote::{quote, ToTokens};
use syn::spanned::Spanned;
use syn::{BinOp, Expr, ExprBinary};

use crate::comm::Mutation;
use crate::transformer::transform_context::TransformContext;
use crate::transformer::transform_info::SharedTransformInfo;

use crate::MutagenRuntimeConfig;

pub struct MutatorBinopEq {}

impl MutatorBinopEq {
    pub fn run<L: PartialEq<R>, R>(
        mutator_id: usize,
        left: L,
        right: R,
        original_op: BinopEq,
        runtime: impl Deref<Target = MutagenRuntimeConfig>,
    ) -> bool {
        runtime.covered(mutator_id);
        let mutations = MutationBinopEq::possible_mutations(original_op);
        if let Some(m) = runtime.get_mutation(mutator_id, &mutations) {
            m.mutate(left, right)
        } else {
            original_op.eq(left, right)
        }
    }

    pub fn transform(
        e: Expr,
        transform_info: &SharedTransformInfo,
        context: &TransformContext,
    ) -> Expr {
        match e {
            Expr::Binary(ExprBinary {
                left,
                right,
                op,
                attrs,
            }) => {
                let op = match op {
                    BinOp::Eq(t) => BinopEqSpanned {
                        op: BinopEq::Eq,
                        span: t.into_token_stream().span(),
                    },
                    BinOp::Ne(t) => BinopEqSpanned {
                        op: BinopEq::Ne,
                        span: t.into_token_stream().span(),
                    },
                    _ => {
                        return Expr::Binary(ExprBinary {
                            left,
                            right,
                            op,
                            attrs,
                        })
                    }
                };
                let mutator_id = transform_info.add_mutations(
                    MutationBinopEq::possible_mutations(op.op)
                        .iter()
                        .map(|m| m.to_mutation(op, context)),
                );

                syn::parse2(quote_spanned! {op.span=>
                    ::mutagen::mutator::MutatorBinopEq::run(
                            #mutator_id,
                            &(#left),
                            &(#right),
                            #op,
                            ::mutagen::MutagenRuntimeConfig::get_default()
                        )
                })
                .expect("transformed code invalid")
            }
            _ => e,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct MutationBinopEq {
    op: BinopEq,
}

impl MutationBinopEq {
    fn possible_mutations(original_op: BinopEq) -> Vec<Self> {
        [BinopEq::Eq, BinopEq::Ne]
            .iter()
            .copied()
            .filter(|&op| op != original_op)
            .map(|op| MutationBinopEq { op })
            .collect()
    }

    fn mutate<L: PartialEq<R>, R>(self, left: L, right: R) -> bool {
        self.op.eq(left, right)
    }

    fn to_mutation(self, original_op: BinopEqSpanned, context: &TransformContext) -> Mutation {
        Mutation::new_spanned(
            context.fn_name.clone(),
            "binop_eq".to_owned(),
            format!("{}", original_op),
            format!("{}", self.op),
            original_op.span,
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BinopEqSpanned {
    op: BinopEq,
    span: Span,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum BinopEq {
    Eq,
    Ne,
}

impl BinopEq {
    fn eq<L: PartialEq<R>, R>(self, left: L, right: R) -> bool {
        match self {
            BinopEq::Eq => left == right,
            BinopEq::Ne => left != right,
        }
    }
}

impl ToTokens for BinopEqSpanned {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // TODO: quote_spanned here
        tokens.extend(quote!(::mutagen::mutator::mutator_binop_eq::BinopEq::));
        tokens.extend(match self.op {
            BinopEq::Eq => quote_spanned!(self.span=> Eq),
            BinopEq::Ne => quote_spanned!(self.span=> Ne),
        })
    }
}

use std::fmt;

impl fmt::Display for BinopEq {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BinopEq::Eq => write!(f, "=="),
            BinopEq::Ne => write!(f, "!="),
        }
    }
}

impl fmt::Display for BinopEqSpanned {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.op)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn eq_inactive() {
        let result = MutatorBinopEq::run(
            1,
            5,
            4,
            BinopEq::Eq,
            &MutagenRuntimeConfig::without_mutation(),
        );
        assert_eq!(result, false);
    }
    #[test]
    fn eq_active() {
        let result = MutatorBinopEq::run(
            1,
            5,
            4,
            BinopEq::Eq,
            &MutagenRuntimeConfig::with_mutation_id(1),
        );
        assert_eq!(result, true);
    }

    #[test]
    fn ne_inactive() {
        let result = MutatorBinopEq::run(
            1,
            5,
            4,
            BinopEq::Ne,
            &MutagenRuntimeConfig::without_mutation(),
        );
        assert_eq!(result, true);
    }
    #[test]
    fn ne_active() {
        let result = MutatorBinopEq::run(
            1,
            5,
            4,
            BinopEq::Ne,
            &MutagenRuntimeConfig::with_mutation_id(1),
        );
        assert_eq!(result, false);
    }
}

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    spanned::Spanned, Error, FnArg, ImplItem, ItemImpl, PathArguments, ReturnType, Signature, Type,
};

use crate::naming::graphql_name;

pub fn expand(input: ItemImpl) -> syn::Result<TokenStream> {
    if input.trait_.is_some() {
        return Err(Error::new(
            input.span(),
            "#[custom_fields] goes on an inherent impl block, not a trait impl",
        ));
    }

    let mut fields = Vec::new();
    for item in &input.items {
        let ImplItem::Fn(function) = item else {
            return Err(Error::new(
                item.span(),
                "a #[custom_fields] impl block holds only resolver functions",
            ));
        };
        fields.push(field(&function.sig)?);
    }

    let self_type = &input.self_ty;
    Ok(quote! {
        #input

        const _: () = {
            impl ::seaography::CustomFields for #self_type {
                fn to_fields(
                    context: &'static ::seaography::BuilderContext,
                ) -> ::std::vec::Vec<::seaography::async_graphql::dynamic::Field> {
                    #[allow(unused_imports)]
                    use ::seaography::{
                        CustomInputType, CustomOutputType, GqlModelHolderType, GqlModelType,
                        GqlScalarValueType,
                    };

                    ::std::vec![#(#fields),*]
                }
            }
        };
    })
}

fn field(signature: &Signature) -> syn::Result<TokenStream> {
    if signature.asyncness.is_none() {
        return Err(Error::new(
            signature.span(),
            "a resolver is `async fn`; it runs inside the request",
        ));
    }

    let function = &signature.ident;
    let field_name = graphql_name(function);
    let output = output_type(signature)?;

    let mut arguments = signature.inputs.iter();
    let Some(context_argument) = arguments.next() else {
        return Err(Error::new(
            signature.span(),
            "a resolver takes `ctx: &ModuleCtx<'_>` as its first argument",
        ));
    };
    require_module_ctx(context_argument)?;

    let mut declarations = Vec::new();
    let mut parsers = Vec::new();
    for argument in arguments {
        let FnArg::Typed(argument) = argument else {
            return Err(Error::new(
                argument.span(),
                "a resolver is an associated function, so it takes no `self`",
            ));
        };
        let syn::Pat::Ident(name) = &*argument.pat else {
            return Err(Error::new(
                argument.pat.span(),
                "a resolver argument is a plain name; it becomes a GraphQL argument",
            ));
        };
        let name = &name.ident;
        let argument_name = graphql_name(name);
        let argument_type = &argument.ty;

        declarations.push(quote! {
            .argument(::seaography::async_graphql::dynamic::InputValue::new(
                #argument_name,
                <#argument_type>::gql_input_type_ref(context),
            ))
        });
        parsers.push(quote! {
            <#argument_type>::parse_value(context, ctx.args.get(#argument_name))
                .map_err(|error| ::module_host::__private::argument_error(
                    #field_name,
                    #argument_name,
                    error,
                ))?
        });
    }

    Ok(quote! {
        ::seaography::async_graphql::dynamic::Field::new(
            #field_name,
            <#output>::gql_output_type_ref(context),
            move |ctx| ::seaography::async_graphql::dynamic::FieldFuture::new(async move {
                let module = ::module_host::__private::module_ctx(ctx.ctx);
                let value = Self::#function(&module #(, #parsers)*).await?;
                ::std::result::Result::Ok(<#output>::gql_field_value(value, context))
            }),
        )
        #(#declarations)*
    })
}

fn require_module_ctx(argument: &FnArg) -> syn::Result<()> {
    let message = "a resolver's first argument is `ctx: &ModuleCtx<'_>` — the only Store access \
                   a Module has";
    let FnArg::Typed(argument) = argument else {
        return Err(Error::new(argument.span(), message));
    };
    let Type::Reference(reference) = &*argument.ty else {
        return Err(Error::new(argument.ty.span(), message));
    };
    let Type::Path(path) = &*reference.elem else {
        return Err(Error::new(argument.ty.span(), message));
    };
    match path.path.segments.last() {
        Some(segment) if segment.ident == "ModuleCtx" => Ok(()),
        _ => Err(Error::new(argument.ty.span(), message)),
    }
}

fn output_type(signature: &Signature) -> syn::Result<TokenStream> {
    let message = "a resolver returns `module_host::Result<T>`, so its output type is declared";
    let ReturnType::Type(_, returned) = &signature.output else {
        return Err(Error::new(signature.span(), message));
    };
    let Type::Path(path) = &**returned else {
        return Err(Error::new(returned.span(), message));
    };
    let Some(segment) = path.path.segments.last() else {
        return Err(Error::new(returned.span(), message));
    };
    if segment.ident != "Result" {
        return Err(Error::new(returned.span(), message));
    }
    let PathArguments::AngleBracketed(parameters) = &segment.arguments else {
        return Err(Error::new(returned.span(), message));
    };
    let Some(parameter) = parameters.args.first() else {
        return Err(Error::new(returned.span(), message));
    };
    Ok(quote!(#parameter))
}

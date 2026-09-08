use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Error, Fields, ItemStruct};

use crate::naming::graphql_name;

pub fn expand(input: ItemStruct) -> syn::Result<TokenStream> {
    let Fields::Named(named) = &input.fields else {
        return Err(Error::new(
            input.ident.span(),
            "an #[output] type needs named fields; each one becomes a GraphQL field",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(Error::new(
            input.generics.span(),
            "an #[output] type cannot be generic; the GraphQL type name would be ambiguous",
        ));
    }

    let ident = &input.ident;
    let type_name = ident.to_string();

    let fields = named
        .named
        .iter()
        .map(|field| {
            let field_ident = field.ident.as_ref().expect("named field");
            let field_type = &field.ty;
            let field_name = graphql_name(field_ident);
            quote! {
                .field(::seaography::async_graphql::dynamic::Field::new(
                    #field_name,
                    <#field_type>::gql_output_type_ref(context),
                    move |ctx| ::seaography::async_graphql::dynamic::FieldFuture::new(async move {
                        let row = ::seaography::try_downcast_ref::<#ident>(ctx.parent_value)?;
                        ::std::result::Result::Ok(
                            <#field_type>::gql_field_value(row.#field_ident.clone(), context),
                        )
                    }),
                ))
            }
        })
        .collect::<Vec<_>>();

    Ok(quote! {
        #input

        const _: () = {
            impl ::seaography::CustomOutputType for #ident {
                fn gql_output_type_ref(
                    _context: &'static ::seaography::BuilderContext,
                ) -> ::seaography::async_graphql::dynamic::TypeRef {
                    ::seaography::async_graphql::dynamic::TypeRef::named_nn(#type_name)
                }

                fn gql_field_value(
                    self,
                    _context: &'static ::seaography::BuilderContext,
                ) -> ::std::option::Option<
                    ::seaography::async_graphql::dynamic::FieldValue<'static>,
                > {
                    ::std::option::Option::Some(
                        ::seaography::async_graphql::dynamic::FieldValue::owned_any(self),
                    )
                }
            }

            impl ::seaography::CustomOutputObject for #ident {
                fn basic_object(
                    context: &'static ::seaography::BuilderContext,
                ) -> ::seaography::async_graphql::dynamic::Object {
                    #[allow(unused_imports)]
                    use ::seaography::{
                        CustomOutputType, GqlModelHolderType, GqlModelType, GqlScalarValueType,
                    };

                    ::seaography::async_graphql::dynamic::Object::new(#type_name)
                        #(#fields)*
                }
            }
        };
    })
}

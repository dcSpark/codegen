use std::fmt::{self, Write};

use crate::formatter::Formatter;

/// Defines a type.
#[derive(Debug, Clone)]
pub struct Type {
    name: String,
    generics: Vec<Type>,
}

fn split_name_and_generic(ast: &syn::Type) -> Type {
    match ast {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            let segments = &path.segments;
            let base_type = segments.iter().map(|seg| seg.ident.to_string()).collect::<Vec<String>>().join("::");
            let mut new_type = Type::new(&base_type);

            if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments { args, .. }) = &segments.last().unwrap().arguments {
                for arg in args.iter() {
                    if let syn::GenericArgument::Type(t) = arg {
                        let generic_type = split_name_and_generic(t);
                        new_type.generic(generic_type);
                    } else {
                        // this isn't correct, but properly parsing the full AST is too tedious and abandoning early here is good enough
                        return Type {
                            name: quote::quote! { #ast }.to_string(),
                            generics: vec![]
                        }
                    }
                }
            };
            new_type
        }
        _ => {
            // this isn't correct, but properly parsing the full AST is too tedious and abandoning early here is good enough
            Type {
                name: quote::quote! { #ast }.to_string(),
                generics: vec![]
            }
        },
    }
}
impl Type {
    /// Return a new type with the given name.
    pub fn new(name: impl ToString) -> Self {
        let name = name.to_string();
        if name.contains('<') {
            split_name_and_generic(&syn::parse_str(&name).unwrap())
        } else {
            Type {
                name,
                generics: Vec::new(),
            }
        }
    }

    /// Returns the name of the type
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Returns the name of the type
    pub fn generics(&self) -> &Vec<Type> {
        &self.generics
    }

    /// Returns the key for sorting
    pub fn key_for_sorting(&self) -> &str {
        match self.name.rfind("::") {
            Some(index) => &self.name[index + 2..],
            None => &self.name,
        }
    }

    /// Add a generic to the type.
    pub fn generic<T>(&mut self, ty: T) -> &mut Self
    where
        T: Into<Type>,
    {
        // Make sure that the name doesn't already include generics
        assert!(
            !self.name.contains("<"),
            "type name already includes generics"
        );

        self.generics.push(ty.into());
        self
    }

    /// Rewrite the `Type` with the provided path
    ///
    /// TODO: Is this needed?
    pub fn path(&self, path: impl ToString) -> Type {
        // TODO: This isn't really correct
        assert!(!self.name.contains("::"));

        let mut name = path.to_string();
        name.push_str("::");
        name.push_str(&self.name);

        Type {
            name,
            generics: self.generics.clone(),
        }
    }

    /// Formats the struct using the given formatter.
    pub fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.name)?;
        Type::fmt_slice(&self.generics, fmt)
    }

    fn fmt_slice(generics: &[Type], fmt: &mut Formatter<'_>) -> fmt::Result {
        if !generics.is_empty() {
            write!(fmt, "<")?;

            for (i, ty) in generics.iter().enumerate() {
                if i != 0 {
                    write!(fmt, ", ")?
                }
                ty.fmt(fmt)?;
            }

            write!(fmt, ">")?;
        }

        Ok(())
    }
}

impl<S: ToString> From<S> for Type {
    fn from(src: S) -> Self {
        Type::new(src)
    }
}

impl<'a> From<&'a Type> for Type {
    fn from(src: &'a Type) -> Self {
        src.clone()
    }
}

#[test]
fn parse_type() {
    {
        let ty = Type::new("u8");
        assert_eq!(ty.name, "u8");
        assert!(ty.generics.is_empty());
    }
}

#[test]
fn parse_generic() {
    {
        let ty = Type::new("Vec<u8>");
        assert_eq!(ty.name, "Vec");
        assert_eq!(ty.generics.iter().map(|generic| generic.name().as_str()).collect::<Vec<&str>>().join(" "), "u8");
    }
    {
        let ty = Type::new("foo::Vec<u8>");
        assert_eq!(ty.name, "foo::Vec");
        assert_eq!(ty.generics.iter().map(|generic| generic.name().as_str()).collect::<Vec<&str>>().join(" "), "u8");
    }
    {
        let ty = Type::new("Vec<Vec<u8>>");
        assert_eq!(ty.name, "Vec");
        assert_eq!(ty.generics.iter().map(|generic| generic.name().as_str()).collect::<Vec<&str>>().join(" "), "Vec");
    }
    {
        let ty = Type::new("BTreeMap<u8, u8>");
        assert_eq!(ty.name, "BTreeMap");
        assert_eq!(ty.generics.iter().map(|generic| generic.name().as_str()).collect::<Vec<&str>>().join(" "), "u8 u8");
    }
    {
        let ty = Type::new("BTreeMap<Vec<u8>, BTreeMap<u64, String>>");
        assert_eq!(ty.name, "BTreeMap");
        assert_eq!(ty.generics.iter().map(|generic| generic.name().as_str()).collect::<Vec<&str>>().join(" "), "Vec BTreeMap");

        let mut ret = String::new();
        ty.fmt(&mut Formatter::new(&mut ret)).unwrap();
        assert_eq!(ret, "BTreeMap<Vec<u8>, BTreeMap<u64, String>>");
    }
    {
        let ty = Type::new("Result<&'a mut Foo<Bar>>");
        assert_eq!(ty.name, "Result");
        assert_eq!(ty.generics.iter().map(|generic| generic.name().as_str()).collect::<Vec<&str>>().join(" "), "& 'a mut Foo < Bar >");
    }
}
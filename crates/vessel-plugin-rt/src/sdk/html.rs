//! `vessel.html` CSS-selector bindings built on `scraper` + `ego_tree` (zero fake `cheerio`).

use std::rc::Rc;

use ego_tree::NodeId;
use rquickjs::function::{FromParams, IntoJsFunc, ParamRequirement, Params};
use rquickjs::{Ctx, Error as QjsError, Function, IntoJs, Object, Result as QjsResult, Value};
use scraper::{CaseSensitivity, ElementRef, Html, Selector};

fn throw_msg<'js>(ctx: &Ctx<'js>, msg: &str) -> QjsError {
    let val = rquickjs::String::from_str(ctx.clone(), msg)
        .map(|s| s.into_value())
        .unwrap_or_else(|_| Value::new_undefined(ctx.clone()));
    ctx.throw(val)
}

fn parse_selector<'js>(ctx: &Ctx<'js>, css: &str) -> Result<Selector, QjsError> {
    Selector::parse(css)
        .map_err(|err| throw_msg(ctx, &format!("invalid CSS selector '{css}': {err:?}")))
}

fn element_from_node<'a>(doc: &'a Html, node_id: NodeId) -> Option<ElementRef<'a>> {
    doc.tree.get(node_id).and_then(ElementRef::wrap)
}

struct SelectFn {
    doc: Rc<Html>,
    node_id: Option<NodeId>,
    single: bool,
}

impl<'js> IntoJsFunc<'js, (String,)> for SelectFn {
    fn param_requirements() -> ParamRequirement {
        <(String,)>::param_requirements()
    }

    fn call(&self, params: Params<'_, 'js>) -> QjsResult<Value<'js>> {
        let ctx = params.ctx().clone();
        let (css,) = <(String,)>::from_params(&mut params.access())?;
        let selector = parse_selector(&ctx, &css)?;

        let matched_ids: Vec<NodeId> = match self.node_id {
            Some(nid) => element_from_node(&self.doc, nid)
                .map(|el| el.select(&selector).map(|m| m.id()).collect())
                .unwrap_or_default(),
            None => self.doc.select(&selector).map(|m| m.id()).collect(),
        };

        if self.single {
            match matched_ids.into_iter().next() {
                Some(nid) => build_element_obj(&ctx, Rc::clone(&self.doc), nid)?.into_js(&ctx),
                None => Ok(Value::new_null(ctx)),
            }
        } else {
            let mut elements = Vec::with_capacity(matched_ids.len());
            for nid in matched_ids {
                elements.push(build_element_obj(&ctx, Rc::clone(&self.doc), nid)?);
            }
            elements.into_js(&ctx)
        }
    }
}

fn build_element_obj<'js>(
    ctx: &Ctx<'js>,
    doc: Rc<Html>,
    node_id: NodeId,
) -> QjsResult<Object<'js>> {
    let obj = Object::new(ctx.clone())?;

    {
        let doc = Rc::clone(&doc);
        obj.set(
            "text",
            Function::new(ctx.clone(), move || -> String {
                element_from_node(&doc, node_id)
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default()
            })?,
        )?;
    }

    {
        let doc = Rc::clone(&doc);
        obj.set(
            "ownText",
            Function::new(ctx.clone(), move || -> String {
                element_from_node(&doc, node_id)
                    .map(|el| {
                        el.children()
                            .filter_map(|child| {
                                child.value().as_text().map(|t| -> &str { t.as_ref() })
                            })
                            .collect::<String>()
                    })
                    .unwrap_or_default()
            })?,
        )?;
    }

    {
        let doc = Rc::clone(&doc);
        obj.set(
            "html",
            Function::new(ctx.clone(), move || -> String {
                element_from_node(&doc, node_id)
                    .map(|el| el.inner_html())
                    .unwrap_or_default()
            })?,
        )?;
    }

    {
        let doc = Rc::clone(&doc);
        obj.set(
            "outerHtml",
            Function::new(ctx.clone(), move || -> String {
                element_from_node(&doc, node_id)
                    .map(|el| el.html())
                    .unwrap_or_default()
            })?,
        )?;
    }

    {
        let doc = Rc::clone(&doc);
        obj.set(
            "attr",
            Function::new(ctx.clone(), move |name: String| -> Option<String> {
                element_from_node(&doc, node_id).and_then(|el| el.attr(&name).map(str::to_owned))
            })?,
        )?;
    }

    {
        let doc = Rc::clone(&doc);
        obj.set(
            "hasClass",
            Function::new(ctx.clone(), move |name: String| -> bool {
                element_from_node(&doc, node_id)
                    .is_some_and(|el| el.value().has_class(&name, CaseSensitivity::CaseSensitive))
            })?,
        )?;
    }

    obj.set(
        "select",
        Function::new(
            ctx.clone(),
            SelectFn {
                doc: Rc::clone(&doc),
                node_id: Some(node_id),
                single: false,
            },
        )?,
    )?;

    obj.set(
        "selectOne",
        Function::new(
            ctx.clone(),
            SelectFn {
                doc,
                node_id: Some(node_id),
                single: true,
            },
        )?,
    )?;

    Ok(obj)
}

fn build_doc_obj<'js>(ctx: &Ctx<'js>, doc: Rc<Html>) -> QjsResult<Object<'js>> {
    let doc_obj = Object::new(ctx.clone())?;

    doc_obj.set(
        "select",
        Function::new(
            ctx.clone(),
            SelectFn {
                doc: Rc::clone(&doc),
                node_id: None,
                single: false,
            },
        )?,
    )?;

    doc_obj.set(
        "selectOne",
        Function::new(
            ctx.clone(),
            SelectFn {
                doc,
                node_id: None,
                single: true,
            },
        )?,
    )?;

    Ok(doc_obj)
}

fn parse_html_fn<'js>(ctx: Ctx<'js>, raw_html: String) -> QjsResult<Object<'js>> {
    let doc = Rc::new(Html::parse_document(&raw_html));
    build_doc_obj(&ctx, doc)
}

/// Attaches `vessel.html.parse(rawHtml)` to the `vessel` host object.
pub(crate) fn attach_html<'js>(ctx: &Ctx<'js>, vessel_obj: &Object<'js>) -> QjsResult<()> {
    let html_obj = Object::new(ctx.clone())?;
    html_obj.set("parse", Function::new(ctx.clone(), parse_html_fn)?)?;
    vessel_obj.set("html", html_obj)?;
    Ok(())
}

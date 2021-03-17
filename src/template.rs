use yarte::Template;

#[derive(Debug, Template)]
#[template(path = "index.html.hbs")]
pub struct Index {
    pub pictures: Vec<String>,
}

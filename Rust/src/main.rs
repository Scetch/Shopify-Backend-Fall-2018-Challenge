///! Brandon Lucier
///! This uses Rust nightly features (try_fold) which will be stable in 1.27

extern crate reqwest;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;

const CART_ENDPOINT: &str = "https://backend-challenge-fall-2018.herokuapp.com/carts.json";

#[derive(Deserialize)]
enum ProductDiscount {
    #[serde(rename = "collection")]
    Collection(String),
    #[serde(rename = "product_value")]
    Value(f32),
}

#[derive(Deserialize)]
#[serde(tag = "discount_type")]
enum DiscountType {
    #[serde(rename = "cart")]
    Cart { cart_value: f32, },
    #[serde(rename = "product")]
    Product(ProductDiscount),
}

#[derive(Deserialize)]
struct Discount {
    id: u32,
    #[serde(flatten)]
    discount_type: DiscountType,
    discount_value: f32,
}

#[derive(Deserialize)]
struct Page {
    products: Vec<Product>,
    pagination: Pagination,
}

#[derive(Deserialize)]
struct Product {
    name: String,
    price: f32,
    collection: Option<String>,
}

#[derive(Deserialize)]
struct Pagination {
    current_page: i32,
    per_page: i32,
    total: i32,
}

/// Returns an iterator that will return a product iterator for each page of the cart with `id`
/// and stop when all pages have been returned or if there was an error returning a previous page
fn get_cart(id: u32) -> impl Iterator<Item = Result<impl Iterator<Item = Product>, reqwest::Error>> {
    (1..).scan((reqwest::Client::new(), false), move |state, page| {
            if state.1 { return None; }

            let (products, stop) = state.0.get(CART_ENDPOINT)
                .query(&[("id", id), ("page", page)])
                .send()
                .and_then(|mut r| r.json::<Page>())
                .map(|Page { products, pagination: Pagination { total, per_page, .. } }| {
                    let stop = page + 1 > (total as f32 / per_page as f32).ceil() as u32;
                    (Ok(products.into_iter()), stop)
                })
                .unwrap_or_else(|e| (Err(e), true));

            state.1 = stop;
            Some(products)
        })
}

/// Calculate the total and total after discount for cart with `id` and type `discount_type`
fn calculate(Discount { id, discount_value, discount_type }: Discount) -> Result<(f32, f32), reqwest::Error> {
    match discount_type {
        DiscountType::Cart { cart_value } => {
            // Only apply the discount if the cart total is greater than cart_value
            get_cart(id)
                .try_fold(0.0, |total, resp| resp.map(|p| {
                    total + p.map(|p| p.price).sum::<f32>()
                }))
                .map(|total| {
                    let d = if total >= cart_value { total - discount_value } else { total };
                    (total, d.max(0.0))
                })
        }
        DiscountType::Product(ty) => {
            // Helper closure to check if the discount should be applied based on
            // the type of product discount
            let apply = |p: &Product| match ty {
                ProductDiscount::Collection(ref col) => {
                    p.collection.as_ref().map(|c| c == col).unwrap_or(false)
                }
                ProductDiscount::Value(val) => p.price >= val,
            };
            
            get_cart(id)
                .try_fold((0.0, 0.0), move |res, resp| resp.map(|p| {
                    p.fold(res, |(t, a), p| {
                            let d = if apply(&p) { p.price - discount_value } else { p.price };
                            (t + p.price, a + d.max(0.0))
                        })
                }))
        }
    }
}

fn main() {
    let dis: Discount = serde_json::from_reader(std::io::stdin())
        .expect("Could not parse discount.");

    let res = calculate(dis)
        .map(|(total, total_after_discount)| {
            json!({ "total": total, "total_after_discount": total_after_discount })
        })
        .expect("Error calculating discount.");

    println!("{}", res);
}

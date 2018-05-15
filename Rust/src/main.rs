extern crate reqwest;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde_json;

use std::io::{ self, prelude::* };

const CART_ENDPOINT: &str = "https://backend-challenge-fall-2018.herokuapp.com/carts.json";

#[derive(Serialize, Deserialize)]
enum ProductDiscount {
    #[serde(rename = "collection")]
    Collection(String),
    #[serde(rename = "product_value")]
    Value(f32),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "discount_type")]
enum DiscountType {
    #[serde(rename = "cart")]
    Cart { cart_value: f32, },
    #[serde(rename = "product")]
    Product(ProductDiscount),
}

#[derive(Serialize, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

fn get_cart(id: u32) -> impl Iterator<Item = Result<impl Iterator<Item = Product>, reqwest::Error>> {
    (1..).scan((reqwest::Client::new(), false), move |state, page| {
            if state.1 { return None; }

            let (products, stop) = state.0.get(CART_ENDPOINT)
                .query(&[("id", id), ("page", page)])
                .send()
                .and_then(|mut r| r.json::<Page>())
                .map(|Page { products, pagination: Pagination { total, per_page, .. }}| {
                    let stop = page + 1 > (total as f32 / per_page as f32).ceil() as u32;
                    (Ok(products.into_iter()), stop)
                })
                .unwrap_or_else(|e| (Err(e), true));

            state.1 = stop;
            Some(products)
        })
}

fn calculate(Discount { id, discount_value, discount_type }: Discount) -> Result<(f32, f32), reqwest::Error> {
    match discount_type {
        DiscountType::Cart { cart_value } => {
            get_cart(id)
                .try_fold(0.0, |total, resp| resp.map(|p| {
                    total + p.map(|p| p.price).sum::<f32>()
                }))
                .map(|total| {
                    let d = if total >= cart_value {
                        total - discount_value
                    } else {
                        total
                    };
                    
                    (total, d.max(0.0))
                })
        }
        DiscountType::Product(ty) => {
            let apply = |p: &Product| match ty {
                ProductDiscount::Collection(ref col) => {
                    p.collection.as_ref().map(|c| c == col).unwrap_or(false)
                }
                ProductDiscount::Value(val) => p.price >= val,
            };
            
            get_cart(id)
                .try_fold((0.0, 0.0), move |res, resp| resp.map(|p| {
                    p.fold(res, |(t, a), p| {
                            let d = if apply(&p) {
                                p.price - discount_value 
                            } else { 
                                p.price 
                            };

                            (t + p.price, a + d.max(0.0))
                        })
                }))
        }
    }
}

fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)
        .expect("Could not read from stdin.");

    let dis = serde_json::from_str::<Discount>(&input)
        .expect("Could not parse discount.");

    let res = calculate(dis)
        .map(|(total, total_after_discount)| {
            json!({
                "total": total,
                "total_after_discount": total_after_discount,
            })
        })
        .expect("Error calculating discount.");

    println!("{}", res);
}

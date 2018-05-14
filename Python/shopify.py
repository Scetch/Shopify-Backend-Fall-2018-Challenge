# Brandon Lucier
# May 10, 2018
# Shopify Fall 2018 Internship Challenge

import json
import math
import urllib.request
import sys

CARTS_ENDPOINT = 'https://backend-challenge-fall-2018.herokuapp.com/carts.json'

# A generator that yields products from all pages associated with an id
def get_cart(id):
    page = 1
    while True:
        resp = urllib.request.urlopen(CARTS_ENDPOINT + "?id=" + str(id) + "&page=" + str(page))
        req = json.loads(resp.read().decode())

        for product in req['products']:
            yield product

        p = req['pagination']
        if page + 1 > math.ceil(p['total'] / p['per_page']):
            break
        else:
            page += 1

# Load the discount data from stdin
dis = json.load(sys.stdin)

total_amount = 0.0
total_after_discount = 0.0

# Calculate the total and total after discount based on the discount
# type provided from stdin
if dis['discount_type'] == 'cart':
    total_amount = sum(map(lambda p: p['price'], get_cart(dis['id'])))

    if total_amount >= dis['cart_value']:
        total_after_discount = total_amount - dis['discount_value']
elif dis['discount_type'] == 'product':
    for p in get_cart(dis['id']):
        total_amount += p['price']

        if (('collection' in dis and p.get('collection') == dis['collection']) or (
            'product_value' in dis and p['price'] >= dis['product_value'])):
            # If we match one of the per-product discount conditions we will apply
            # the discount.
            total_after_discount += max(p['price'] - dis['discount_value'], 0)
        else:
            total_after_discount += p['price']

print('{\n  "total_amount": ' + str(total_amount) + ',\n  "total_after_discount": ' + str(total_after_discount) + '\n}')
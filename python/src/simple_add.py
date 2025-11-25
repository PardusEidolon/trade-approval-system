import uniffi_py as rust_py

x = rust_py.add(2, 2)

print(f"rust add function returns {x}")

# create a new sled db if it doesnt exist. input takes a string which is coerced into a path
rust_py.sled_db()

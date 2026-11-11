def greet(name):
    cleaned = prepare_name(name)
    return message(cleaned)


def prepare_name(name):
    return name.strip()


def message(name):
    return f"Hello, {name}!"


def main():
    print(greet("  Ada  "))


if __name__ == "__main__":
    main()

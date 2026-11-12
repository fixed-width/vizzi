module Greetings
  class Greeter
    def greet(name)
      cleaned = prepare_name(name)
      message(cleaned)
    end

    private

    def prepare_name(name)
      name.strip
    end

    def message(name)
      "Hello, #{name}!"
    end
  end
end

puts Greetings::Greeter.new.greet("  Ada  ") if $PROGRAM_NAME == __FILE__

// A small interpreter: arithmetic expressions written out, tokenized and evaluated.
package main

import (
	"fmt"
	"os"
	"strconv"
	"unicode"
)

const modulus = 1000003

// modulo is Python's `%`, whose result takes the divisor's sign.
func modulo(a, m int) int { return ((a % m) + m) % m }

func expression(seed int) string {
	parts := make([]string, 12)
	for k := range parts {
		value := strconv.Itoa((seed*31+k*17)%97 + 1)
		switch k % 4 {
		case 0:
			parts[k] = "(" + value
		case 3:
			parts[k] = value + ")"
		default:
			parts[k] = value
		}
	}
	operators := []string{"+", "*", "-", "+"}
	text := parts[0]
	for k := 1; k < 12; k++ {
		text += " " + operators[(seed+k)%4] + " " + parts[k]
	}
	return text
}

func tokens(text string) []string {
	var found []string
	number := ""
	for _, c := range text {
		if unicode.IsDigit(c) {
			number += string(c)
			continue
		}
		if number != "" {
			found = append(found, number)
			number = ""
		}
		if c != ' ' {
			found = append(found, string(c))
		}
	}
	if number != "" {
		found = append(found, number)
	}
	return found
}

type parser struct {
	items    []string
	position int
}

func (p *parser) peek() string {
	if p.position < len(p.items) {
		return p.items[p.position]
	}
	return ""
}

func (p *parser) take() string {
	p.position++
	return p.items[p.position-1]
}

func (p *parser) sum() int {
	value := p.product()
	for p.peek() == "+" || p.peek() == "-" {
		if p.take() == "+" {
			value = modulo(value+p.product(), modulus)
		} else {
			value = modulo(value-p.product(), modulus)
		}
	}
	return value
}

func (p *parser) product() int {
	value := p.atom()
	for p.peek() == "*" {
		p.take()
		value = modulo(value*p.atom(), modulus)
	}
	return value
}

func (p *parser) atom() int {
	item := p.take()
	if item == "(" {
		value := p.sum()
		p.take()
		return value
	}
	value, _ := strconv.Atoi(item)
	return value
}

func main() {
	n, _ := strconv.Atoi(os.Args[1])
	total := 0
	for i := 0; i < n; i++ {
		p := &parser{items: tokens(expression(i))}
		total = modulo(total+p.sum(), modulus)
	}
	fmt.Println(total)
}

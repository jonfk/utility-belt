package main

type CharType int

const (
	NumberCharType CharType = iota
	SpecialCharType
	UpperCharType
	LowerCharType
	NonVisibleCharType
	InvalidCharType
)

func GetCharType(char int32) CharType {
	switch {
	case char < 0:
		return InvalidCharType
	case char >= 0 && char < 32:
		return NonVisibleCharType
	case char >= 32 && char < 48:
		return SpecialCharType
	case char >= 48 && char < 58:
		return NumberCharType
	case char >= 58 && char < 65:
		return SpecialCharType
	case char >= 65 && char < 91:
		return UpperCharType
	case char >= 91 && char < 97:
		return SpecialCharType
	case char >= 97 && char < 123:
		return LowerCharType
	case char >= 123 && char < 127:
		return SpecialCharType
	case char == 127:
		return NonVisibleCharType
	case char > 127:
		return InvalidCharType
	default:
		return InvalidCharType
	}
}

func containsCharType(charType CharType, types []CharType) bool {
	for _, x := range types {
		if x == charType {
			return true
		}
	}
	return false
}

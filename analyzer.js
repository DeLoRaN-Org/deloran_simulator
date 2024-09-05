const fs = require('fs')


let file_path = process.argv[2]
let rows = fs.readFileSync(file_path).toString().split('\n').filter(r => r.length > 0)


let max = 0
let min = 999999999999
let above_2 = 0
let base_tmst = 0
let latest_tmst = 0


let average_rtt = rows.map(row => {
  let v = Number(row.split(',')[1])
  if(v > max) max = v
  if(v < min) min = v
  if(v > 1999) above_2 += 1
  if(base_tmst == 0) base_tmst = Number(row.split(',')[0])
  latest_tmst = Number(row.split(',')[0])
  return v
}).reduce((acc, curr) => acc + curr, 0) / rows.length

let variance = rows.map(row => {
  let v = Number(row.split(',')[1])
  return (v - average_rtt)**2
}).reduce((acc, curr) => acc + curr, 0) / (rows.length - 1)


console.log(`Above 5s: ${above_2}, total: ${rows.length}`)
console.log(`Average Rtt: ${average_rtt}`)
console.log(`Total Num: ${rows.length}`)
console.log(`Max Rtt: ${max}`)
console.log(`Min Rtt: ${min}`)
console.log(`Variance: ${Math.sqrt(variance)}`)
console.log(`Duration: ${(latest_tmst - base_tmst) / 60000  }`)

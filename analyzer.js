const fs = require('fs')

let rows = fs.readFileSync('rtt_times.csv').toString().split('\n')


let max = 0
let min = 999999999999
let above_6 = 0

let average_rtt = rows.filter(r => r.length > 0).map(row => {
  let v = Number(row.split(',')[2])
  if(v > max) max = v
  if(v < min) min = v
  if(v > 6000) above_6 += 1
  return v
}).reduce((acc, curr) => acc + curr, 0) / rows.length

let variance = rows.filter(r => r.length > 0).map(row => {
  let v = Number(row.split(',')[2])
  return (v - average_rtt)**2
}).reduce((acc, curr) => acc + curr, 0) / (rows.length - 1)


console.log(`Above 5s: ${above_6}, total: ${rows.filter(r => r.length > 0).length}`)
console.log(`Average Rtt: ${average_rtt}`)
console.log(`Total Num: ${rows.length}`)
console.log(`Max Rtt: ${max}`)
console.log(`Min Rtt: ${min}`)
console.log(`Variance: ${Math.sqrt(variance)}`)
